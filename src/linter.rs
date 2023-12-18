// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::ast_parser::parse_program;
use crate::context::Context;
use crate::control_flow::ControlFlow;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::{
  parse_file_ignore_directives, parse_line_ignore_directives,
};
use crate::performance_mark::PerformanceMark;
use crate::rules::{ban_unknown_rule_code::BanUnknownRuleCode, LintRule};
use deno_ast::Diagnostic;
use deno_ast::MediaType;
use deno_ast::ParsedSource;
use deno_ast::Scope;

pub struct LinterBuilder {
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  media_type: MediaType,
  rules: Vec<&'static dyn LintRule>,
}

impl Default for LinterBuilder {
  fn default() -> Self {
    Self {
      ignore_file_directive: "deno-lint-ignore-file".to_string(),
      ignore_diagnostic_directive: "deno-lint-ignore".to_string(),
      media_type: MediaType::TypeScript,
      rules: Vec::new(),
    }
  }
}

impl LinterBuilder {
  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directive,
      self.ignore_diagnostic_directive,
      self.media_type,
      self.rules,
    )
  }

  /// Set name for directive that can be used to skip linting file.
  ///
  /// Defaults to "deno-lint-ignore-file".
  pub fn ignore_file_directive(mut self, directive: &str) -> Self {
    self.ignore_file_directive = directive.to_owned();
    self
  }

  /// Set name for directive that can be used to ignore next line.
  ///
  /// Defaults to "deno-lint-ignore".
  pub fn ignore_diagnostic_directive(mut self, directive: &str) -> Self {
    self.ignore_diagnostic_directive = directive.to_owned();
    self
  }

  /// Set media type of a file to be linted.
  ///
  /// Defaults to `MediaType::TypeScript`
  pub fn media_type(mut self, media_type: MediaType) -> Self {
    self.media_type = media_type;
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

pub struct Linter {
  ctx: LinterContext,
}

/// TODO
pub struct LinterContext {
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  // TODO(bartlomieju): this should be on `FileContext`
  media_type: MediaType,
  check_unknown_rules: bool,
  /// Rules are sorted by priority
  rules: Vec<&'static dyn LintRule>,
}

impl LinterContext {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    media_type: MediaType,
    mut rules: Vec<&'static dyn LintRule>,
  ) -> Self {
    crate::rules::sort_rules_by_priority(&mut rules);
    let check_unknown_rules = rules
      .iter()
      .any(|a| a.code() == (BanUnknownRuleCode).code());

    LinterContext {
      ignore_file_directive,
      ignore_diagnostic_directive,
      media_type,
      check_unknown_rules,
      rules,
    }
  }
}

impl Linter {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    media_type: MediaType,
    rules: Vec<&'static dyn LintRule>,
  ) -> Self {
    let ctx = LinterContext::new(
      ignore_file_directive,
      ignore_diagnostic_directive,
      media_type,
      rules,
    );

    Linter { ctx }
  }

  pub fn lint(
    mut self,
    file_name: String,
    source_code: String,
  ) -> Result<(ParsedSource, Vec<LintDiagnostic>), Diagnostic> {
    let _mark = PerformanceMark::new("Linter::lint");

    let syntax = deno_ast::get_syntax(self.ctx.media_type);
    let parse_result = {
      let _mark = PerformanceMark::new("ast_parser.parse_program");
      parse_program(&file_name, syntax, source_code)
    };

    let parsed_source = parse_result?;
    let diagnostics = self.lint_program(&parsed_source);

    Ok((parsed_source, diagnostics))
  }

  pub fn lint_with_ast(
    mut self,
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_with_ast");
    self.lint_program(parsed_source)
  }

  fn filter_diagnostics(&self, mut context: Context) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::filter_diagnostics");

    let mut filtered_diagnostics = context.check_ignore_directive_usage();
    // Run `ban-unknown-rule-code`
    filtered_diagnostics.extend(context.ban_unknown_rule_code());
    // Run `ban-unused-ignore`
    filtered_diagnostics.extend(context.ban_unused_ignore(&self.ctx.rules));
    filtered_diagnostics.sort_by_key(|d| d.range.start.line_index);

    filtered_diagnostics
  }

  fn lint_program(
    &mut self,
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    let _mark = PerformanceMark::new("Linter::lint_module");

    let control_flow = ControlFlow::analyze(parsed_source);
    let diagnostics = parsed_source.with_view(|pg| {
      let file_ignore_directive =
        parse_file_ignore_directives(&self.ctx.ignore_file_directive, pg);

      // If a global ignore directive that has no codes specified exists, we must skip linting on
      // this file.
      if matches!(file_ignore_directive, Some(ref file_ignore) if file_ignore.ignore_all())
      {
        return vec![];
      }

      let line_ignore_directives =
        parse_line_ignore_directives(&self.ctx.ignore_diagnostic_directive, pg);

      let scope = Scope::analyze(pg);

      let mut context = Context::new(
        parsed_source.clone(),
        self.ctx.media_type,
        pg,
        file_ignore_directive,
        line_ignore_directives,
        scope,
        control_flow,
        self.ctx.check_unknown_rules,
      );

      // Run builtin rules
      for rule in self.ctx.rules.iter() {
        rule.lint_program_with_ast_view(&mut context, pg);
      }

      self.filter_diagnostics(context)
    });

    diagnostics
  }
}
