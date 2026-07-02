// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::ast_parser::parse_program;
use crate::context::Context;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::parse_file_ignore_directives;
use crate::performance_mark::PerformanceMark;
use crate::rules::{ban_unknown_rule_code::BanUnknownRuleCode, LintRule};
use deno_ast::diagnostics::Diagnostic;
use deno_ast::oxc::allocator::Allocator;
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
pub type ExternalLinterCb =
  Arc<dyn for<'a> Fn(&'a ParsedSource<'a>) -> Option<ExternalLinterResult>>;

pub struct LintFileOptions {
  pub specifier: ModuleSpecifier,
  pub source_code: String,
  pub media_type: MediaType,
  pub config: LintConfig,
  pub external_linter: Option<ExternalLinterCb>,
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
  /// Returns `Vec<LintDiagnostic>`. If you need the `ParsedSource`, parse it
  /// separately and use `lint_with_ast`.
  pub fn lint_file(
    &self,
    options: LintFileOptions,
  ) -> Result<Vec<LintDiagnostic>, ParseDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint");

    let allocator = Allocator::default();
    let parsed_source = {
      let _mark = PerformanceMark::new("ast_parser.parse_program");
      parse_program(
        &allocator,
        options.specifier,
        options.media_type,
        options.source_code,
      )
    }?;

    let diagnostics =
      self.lint_inner(&parsed_source, &options.config, options.external_linter);

    Ok(diagnostics)
  }

  /// Lint an already parsed file.
  ///
  /// This method is useful in context where the file is already parsed for other
  /// purposes like transpilation or LSP analysis.
  pub fn lint_with_ast<'a>(
    &self,
    parsed_source: &'a ParsedSource<'a>,
    config: LintConfig,
    maybe_external_linter: Option<ExternalLinterCb>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_with_ast");
    self.lint_inner(parsed_source, &config, maybe_external_linter)
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

  fn lint_inner<'a>(
    &self,
    parsed_source: &'a ParsedSource<'a>,
    config: &LintConfig,
    maybe_external_linter: Option<ExternalLinterCb>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_inner");

    let file_ignore_directive = parse_file_ignore_directives(
      self.ctx.ignore_file_directive,
      parsed_source,
    );
    if let Some(ignore_directive) = file_ignore_directive.as_ref() {
      if ignore_directive.ignore_all() {
        return vec![];
      }
    }

    let mut context =
      Context::new(&self.ctx, parsed_source, config, file_ignore_directive);

    let program = parsed_source.program();

    // Run configured lint rules.
    for rule in self.ctx.rules.iter() {
      rule.lint_program_with_ast_view(&mut context, program);
    }

    let mut external_rule_codes = vec![];
    if let Some(cb) = maybe_external_linter {
      if let Some(external_linter_result) = cb(parsed_source) {
        context.add_external_diagnostics(&external_linter_result.diagnostics);
        external_rule_codes = external_linter_result.rules;
      }
    }

    self.collect_diagnostics(context, external_rule_codes)
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
    let diagnostics = linter
      .lint_file(LintFileOptions {
        specifier,
        source_code: source.to_string(),
        media_type,
        config: LintConfig {
          default_jsx_factory: None,
          default_jsx_fragment_factory: None,
        },
        external_linter: None,
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
}
