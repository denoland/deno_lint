// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::ast_parser::parse_program;
use crate::context::Context;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::parse_file_ignore_directives;
use crate::performance_mark::PerformanceMark;
use crate::rules::{ban_unknown_rule_code::BanUnknownRuleCode, LintRule};
use deno_ast::diagnostics::Diagnostic;
use deno_ast::MediaType;
use deno_ast::ParsedSource;
use deno_ast::{ModuleSpecifier, ParseDiagnostic};
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

pub struct LinterOptions {
  /// Rules to lint with.
  pub rules: Vec<Box<dyn LintRule>>,
  /// Collection of all the lint rule codes.
  pub all_rule_codes: HashSet<Cow<'static, str>>,
  /// Defaults to "deno-lint-ignore-file"
  pub custom_ignore_file_directive: Option<&'static str>,
  /// Defaults to "deno-lint-ignore"
  pub custom_ignore_diagnostic_directive: Option<&'static str>,
}

/// A linter instance.
#[derive(Debug)]
pub struct Linter {
  ctx: LinterContext,
}

/// A struct defining configuration of a `Linter` instance.
///
/// This struct is passed along and used to construct a specific file context,
/// just before a particular file is linted.
#[derive(Debug)]
pub(crate) struct LinterContext {
  pub ignore_file_directive: &'static str,
  pub ignore_diagnostic_directive: &'static str,
  pub check_unknown_rules: bool,
  /// Rules are sorted by priority
  pub rules: Vec<Box<dyn LintRule>>,
  pub all_rule_codes: HashSet<Cow<'static, str>>,
}

impl LinterContext {
  fn new(options: LinterOptions) -> Self {
    let mut rules = options.rules;
    crate::rules::sort_rules_by_priority(&mut rules);
    let check_unknown_rules = rules
      .iter()
      .any(|a| a.code() == (BanUnknownRuleCode).code());

    LinterContext {
      ignore_file_directive: options
        .custom_ignore_file_directive
        .unwrap_or("deno-lint-ignore-file"),
      ignore_diagnostic_directive: options
        .custom_ignore_diagnostic_directive
        .unwrap_or("deno-lint-ignore"),
      check_unknown_rules,
      rules,
      all_rule_codes: options.all_rule_codes,
    }
  }
}

#[derive(Default)]
pub struct ExternalLinterResult {
  pub diagnostics: Vec<LintDiagnostic>,
  pub rules: Vec<Cow<'static, str>>,
}

/// Perform a run of "external linter" on a parsed source file.
///
/// Since we are working on an already parsed file, this callback
/// is infallible. If an error handling needs to be performed by the
/// external linter, it should be handled externally bt that linter,
///  and an empty [`ExternalLinterResult`] should be returned.
pub type ExternalLinterCb =
  Arc<dyn Fn(ParsedSource) -> Option<ExternalLinterResult>>;

pub struct LintFileOptions {
  pub specifier: ModuleSpecifier,
  pub source_code: String,
  pub media_type: MediaType,
  pub config: LintConfig,
  pub external_linter: Option<ExternalLinterCb>,
  /// Optional mapping info to translate diagnostic positions/fixes back to
  /// coordinates in the original, parent file (e.g. for Vue or Svelte SFCs).
  pub source_mapping: Option<SourceMapping>,
}

/// Mapping information used to translate source positions/ranges and fixes of
/// diagnostics generated for an extracted code segment (like a `<script>` tag)
/// back to positions relative to the original, parent source file.
#[derive(Debug, Clone)]
pub struct SourceMapping {
  /// The full source code of the parent file.
  pub original_source: String,
  /// The start index of the extracted code block relative to the parent file (in bytes).
  pub byte_offset: usize,
}

#[derive(Debug, Clone)]
pub struct LintConfig {
  pub default_jsx_factory: Option<String>,
  pub default_jsx_fragment_factory: Option<String>,
}

impl Linter {
  pub fn new(options: LinterOptions) -> Self {
    let ctx = LinterContext::new(options);

    Linter { ctx }
  }

  /// Lint a single file.
  ///
  /// Returns `ParsedSource` and `Vec<ListDiagnostic>`, so the file can be
  /// processed further without having to be parsed again.
  ///
  /// If you have an already parsed file, use `Linter::lint_with_ast` instead.
  pub fn lint_file(
    &self,
    options: LintFileOptions,
  ) -> Result<(ParsedSource, Vec<LintDiagnostic>), ParseDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint");

    let parse_result = {
      let _mark = PerformanceMark::new("ast_parser.parse_program");
      parse_program(options.specifier, options.media_type, options.source_code)
    };

    let parsed_source = parse_result?;
    let diagnostics = self.lint_inner(
      &parsed_source,
      options.config.default_jsx_factory,
      options.config.default_jsx_fragment_factory,
      options.external_linter,
      options.source_mapping,
    );

    Ok((parsed_source, diagnostics))
  }

  /// Lint an already parsed file.
  ///
  /// This method is useful in context where the file is already parsed for other
  /// purposes like transpilation or LSP analysis.
  ///
  /// `maybe_source_mapping` is an optional parameter to map diagnostic ranges
  /// and fixes back to the coordinates of the original template file.
  pub fn lint_with_ast(
    &self,
    parsed_source: &ParsedSource,
    config: LintConfig,
    maybe_external_linter: Option<ExternalLinterCb>,
    maybe_source_mapping: Option<SourceMapping>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_with_ast");
    self.lint_inner(
      parsed_source,
      config.default_jsx_factory,
      config.default_jsx_fragment_factory,
      maybe_external_linter,
      maybe_source_mapping,
    )
  }

  // TODO(bartlomieju): this struct does too much - not only it checks for ignored
  // lint rules, it also runs 2 additional rules. These rules should be rewritten
  // to use a regular way of writing a rule and not live on the `Context` struct.
  fn collect_diagnostics(
    &self,
    mut context: Context,
    external_rule_codes: Vec<Cow<'static, str>>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::collect_diagnostics");

    let mut diagnostics = context.check_ignore_directive_usage();

    let mut all_rules = self.ctx.all_rule_codes.clone();
    all_rules.extend(external_rule_codes.iter().cloned());
    let enabled_rules: HashSet<Cow<'static, str>> = external_rule_codes
      .into_iter()
      .chain(self.ctx.rules.iter().map(|r| r.code().into()))
      .collect();

    // Run `ban-unknown-rule-code`
    diagnostics.extend(context.ban_unknown_rule_code(&all_rules));
    // Run `ban-unused-ignore`
    diagnostics.extend(context.ban_unused_ignore(&enabled_rules));

    // Finally sort by position the diagnostics originates on then by code
    diagnostics.sort_by(|a, b| {
      let a_range = a.range.as_ref().map(|r| r.range.start);
      let b_range = b.range.as_ref().map(|r| r.range.start);
      match a_range.cmp(&b_range) {
        std::cmp::Ordering::Equal => a.code().cmp(&b.code()),
        cmp => cmp,
      }
    });

    diagnostics
  }

  fn lint_inner(
    &self,
    parsed_source: &ParsedSource,
    default_jsx_factory: Option<String>,
    default_jsx_fragment_factory: Option<String>,
    maybe_external_linter: Option<ExternalLinterCb>,
    maybe_source_mapping: Option<SourceMapping>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_inner");

    let mut diagnostics = parsed_source.with_view(|pg| {
      // If a top-level ignore directive exists, eg:
      // ```
      //   // deno-lint-ignore-file
      // ```
      // and there's no particular rule(s) specified, eg:
      // ```
      //   // deno-lint-ignore-file no-undefined
      // ```
      // we want to ignore the whole file.
      //
      // That means we want to return no diagnostics for a particular file, so
      // we're gonna check if the file should be ignored, before performing
      // other expensive work like scope or control-flow analysis.
      let file_ignore_directive =
        parse_file_ignore_directives(self.ctx.ignore_file_directive, pg);
      if let Some(ignore_directive) = file_ignore_directive.as_ref() {
        if ignore_directive.ignore_all() {
          return vec![];
        }
      }

      // TODO(bartlomieju): rename to `FileContext`? It would be a very noisy
      // change, but "Context" is so ambiguous.
      let mut context = Context::new(
        &self.ctx,
        parsed_source.clone(),
        pg,
        file_ignore_directive,
        default_jsx_factory,
        default_jsx_fragment_factory,
      );

      // Run configured lint rules.
      for rule in self.ctx.rules.iter() {
        rule.lint_program_with_ast_view(&mut context, pg);
      }

      let mut external_rule_codes = vec![];
      if let Some(cb) = maybe_external_linter {
        if let Some(external_linter_result) = cb(parsed_source.clone()) {
          context.add_external_diagnostics(&external_linter_result.diagnostics);
          external_rule_codes = external_linter_result.rules;
        }
      }

      self.collect_diagnostics(context, external_rule_codes)
    });

    if let Some(mapping) = maybe_source_mapping {
      let original_text_info =
        deno_ast::SourceTextInfo::from_string(mapping.original_source);
      for diagnostic in &mut diagnostics {
        if let Some(ref mut range) = diagnostic.range {
          range.range = deno_ast::SourceRange {
            start: range.range.start + mapping.byte_offset,
            end: range.range.end + mapping.byte_offset,
          };
          range.text_info = original_text_info.clone();
        }

        for fix in &mut diagnostic.details.fixes {
          for change in &mut fix.changes {
            change.range = deno_ast::SourceRange {
              start: change.range.start + mapping.byte_offset,
              end: change.range.end + mapping.byte_offset,
            };
          }
        }
      }

      diagnostics.sort_by(|a, b| {
        let a_range = a.range.as_ref().map(|r| r.range.start);
        let b_range = b.range.as_ref().map(|r| r.range.start);
        match a_range.cmp(&b_range) {
          std::cmp::Ordering::Equal => a.code().cmp(&b.code()),
          cmp => cmp,
        }
      });
    }

    diagnostics
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::rules::no_debugger::NoDebugger;

  fn lint_with_directives(
    source: &str,
    custom_ignore_diagnostic_directive: Option<&'static str>,
  ) -> Vec<LintDiagnostic> {
    let linter = Linter::new(LinterOptions {
      rules: vec![Box::new(NoDebugger)],
      all_rule_codes: [Cow::from("no-debugger")].into_iter().collect(),
      custom_ignore_file_directive: None,
      custom_ignore_diagnostic_directive,
    });
    let specifier = ModuleSpecifier::parse("file:///foo.ts").unwrap();
    let media_type = MediaType::from_specifier(&specifier);
    let (_, diagnostics) = linter
      .lint_file(LintFileOptions {
        specifier,
        source_code: source.to_string(),
        media_type,
        config: LintConfig {
          default_jsx_factory: None,
          default_jsx_fragment_factory: None,
        },
        external_linter: None,
        source_mapping: None,
      })
      .unwrap();
    diagnostics
  }

  // Regression test for #1475: a configured `custom_ignore_diagnostic_directive`
  // must actually be honored for line-level ignores. Previously the linter
  // mistakenly initialized `ignore_diagnostic_directive` from
  // `custom_ignore_file_directive`, so the custom value was never used.
  #[test]
  fn custom_ignore_diagnostic_directive_is_respected() {
    // Sanity check: with no ignore comment the rule fires.
    assert_eq!(
      lint_with_directives("debugger;", Some("custom-ignore")).len(),
      1
    );

    // The custom directive suppresses the diagnostic.
    let source = "// custom-ignore no-debugger\ndebugger;";
    assert!(lint_with_directives(source, Some("custom-ignore")).is_empty());

    // The default `deno-lint-ignore` is no longer recognized once a custom
    // directive is configured, so the diagnostic still fires.
    let source = "// deno-lint-ignore no-debugger\ndebugger;";
    assert_eq!(lint_with_directives(source, Some("custom-ignore")).len(), 1);
  }

  // With no custom directive, the default `deno-lint-ignore` still works.
  #[test]
  fn default_ignore_diagnostic_directive_is_respected() {
    let source = "// deno-lint-ignore no-debugger\ndebugger;";
    assert!(lint_with_directives(source, None).is_empty());
  }

  /// Verifies that linting an extracted script block from a markup template file
  /// (e.g. Vue SFC or Svelte component) correctly translates the diagnostic positions,
  /// line/column indexes, and source text info back to the parent file coordinates.
  #[test]
  fn sfc_source_mapping_works() {
    // A simulated Vue Single File Component (SFC)
    let original = "<template>\n  <div>Hello</div>\n</template>\n<script lang=\"ts\">\n  debugger;\n</script>";
    // The extracted script block code to be linted
    let script = "  debugger;";
    // Calculate the start position of this block within the parent SFC file
    let byte_offset = original.find(script).unwrap();

    let linter = Linter::new(LinterOptions {
      rules: vec![Box::new(NoDebugger)],
      all_rule_codes: [Cow::from("no-debugger")].into_iter().collect(),
      custom_ignore_file_directive: None,
      custom_ignore_diagnostic_directive: None,
    });
    let specifier = ModuleSpecifier::parse("file:///foo.vue.ts").unwrap();
    let (_, diagnostics) = linter
      .lint_file(LintFileOptions {
        specifier,
        source_code: script.to_string(),
        media_type: MediaType::TypeScript,
        config: LintConfig {
          default_jsx_factory: None,
          default_jsx_fragment_factory: None,
        },
        external_linter: None,
        source_mapping: Some(SourceMapping {
          original_source: original.to_string(),
          byte_offset,
        }),
      })
      .unwrap();

    assert_eq!(diagnostics.len(), 1);
    let diag = &diagnostics[0];
    let range = diag.range.as_ref().unwrap();

    // Check that start offset points to the original template file
    assert_eq!(
      range.range.start,
      deno_ast::StartSourcePos::START_SOURCE_POS + byte_offset + 2
    );

    // Check that the line is mapped correctly (line 5 in the Vue SFC, so index 4)
    let line_and_col = range.text_info.line_and_column_index(range.range.start);
    assert_eq!(line_and_col.line_index, 4);
    assert_eq!(line_and_col.column_index, 2);
  }
}
