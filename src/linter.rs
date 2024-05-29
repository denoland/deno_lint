// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::ast_parser::parse_program;
use crate::context::Context;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::parse_file_ignore_directives;
use crate::performance_mark::PerformanceMark;
use crate::rules::{ban_unknown_rule_code::BanUnknownRuleCode, LintRule};
use deno_ast::MediaType;
use deno_ast::ParsedSource;
use deno_ast::{ModuleSpecifier, ParseDiagnostic};
use std::sync::Arc;

pub struct LinterBuilder {
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  rules: Vec<&'static dyn LintRule>,
}

impl Default for LinterBuilder {
  fn default() -> Self {
    Self {
      ignore_file_directive: "deno-lint-ignore-file".to_string(),
      ignore_diagnostic_directive: "deno-lint-ignore".to_string(),
      rules: Vec::new(),
    }
  }
}

impl LinterBuilder {
  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directive,
      self.ignore_diagnostic_directive,
      self.rules,
    )
  }

  /// Set name for directive that can be used to skip linting file.
  ///
  /// Defaults to "deno-lint-ignore-file".
  pub fn ignore_file_directive(mut self, directive: &str) -> Self {
    directive.clone_into(&mut self.ignore_file_directive);
    self
  }

  /// Set name for directive that can be used to ignore next line.
  ///
  /// Defaults to "deno-lint-ignore".
  pub fn ignore_diagnostic_directive(mut self, directive: &str) -> Self {
    directive.clone_into(&mut self.ignore_diagnostic_directive);
    self
  }

  /// Set a list of rules that will be used for linting.
  ///
  /// Defaults to empty list (no rules will be run by default).
  pub fn rules(mut self, rules: Vec<&'static dyn LintRule>) -> Self {
    self.rules = rules;
    self
  }
}

/// A linter instance. It can be cheaply cloned and shared between threads.
#[derive(Clone)]
pub struct Linter {
  ctx: Arc<LinterContext>,
}

/// A struct defining configuration of a `Linter` instance.
///
/// This struct is passed along and used to construct a specific file context,
/// just before a particular file is linted.
pub struct LinterContext {
  pub ignore_file_directive: String,
  pub ignore_diagnostic_directive: String,
  pub check_unknown_rules: bool,
  /// Rules are sorted by priority
  pub rules: Vec<&'static dyn LintRule>,
}

impl LinterContext {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    mut rules: Vec<&'static dyn LintRule>,
  ) -> Self {
    crate::rules::sort_rules_by_priority(&mut rules);
    let check_unknown_rules = rules
      .iter()
      .any(|a| a.code() == (BanUnknownRuleCode).code());

    LinterContext {
      ignore_file_directive,
      ignore_diagnostic_directive,
      check_unknown_rules,
      rules,
    }
  }
}

pub struct LintFileOptions {
  pub specifier: ModuleSpecifier,
  pub source_code: String,
  pub media_type: MediaType,
  pub default_jsx_factory: Option<String>,
  pub default_jsx_fragment_factory: Option<String>,
}

impl Linter {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    rules: Vec<&'static dyn LintRule>,
  ) -> Self {
    let ctx = LinterContext::new(
      ignore_file_directive,
      ignore_diagnostic_directive,
      rules,
    );

    Linter { ctx: Arc::new(ctx) }
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
      options.default_jsx_factory,
      options.default_jsx_fragment_factory,
    );

    Ok((parsed_source, diagnostics))
  }

  /// Lint an already parsed file.
  ///
  /// This method is useful in context where the file is already parsed for other
  /// purposes like transpilation or LSP analysis.
  pub fn lint_with_ast(
    &self,
    parsed_source: &ParsedSource,
    default_jsx_factory: Option<String>,
    default_jsx_fragment_factory: Option<String>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_with_ast");
    self.lint_inner(
      parsed_source,
      default_jsx_factory,
      default_jsx_fragment_factory,
    )
  }

  // TODO(bartlomieju): this struct does too much - not only it checks for ignored
  // lint rules, it also runs 2 additional rules. These rules should be rewritten
  // to use a regular way of writing a rule and not live on the `Context` struct.
  fn collect_diagnostics(&self, mut context: Context) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::collect_diagnostics");

    let mut diagnostics = context.check_ignore_directive_usage();
    // Run `ban-unknown-rule-code`
    diagnostics.extend(context.ban_unknown_rule_code());
    // Run `ban-unused-ignore`
    diagnostics.extend(context.ban_unused_ignore(&self.ctx.rules));

    // Finally sort by position the diagnostics originates on
    diagnostics.sort_by_key(|d| d.range.start);

    diagnostics
  }

  fn lint_inner(
    &self,
    parsed_source: &ParsedSource,
    default_jsx_factory: Option<String>,
    default_jsx_fragment_factory: Option<String>,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_inner");

    let diagnostics = parsed_source.with_view(|pg| {
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
        parse_file_ignore_directives(&self.ctx.ignore_file_directive, pg);
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

      self.collect_diagnostics(context)
    });

    diagnostics
  }
}
