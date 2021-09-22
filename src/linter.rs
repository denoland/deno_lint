// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::ast_parser::get_default_ts_config;
use crate::ast_parser::AstParser;
use crate::ast_parser::SwcDiagnostic;
use crate::context::Context;
use crate::control_flow::ControlFlow;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::{
  parse_file_ignore_directives, parse_line_ignore_directives,
};
use crate::rules::LintRule;
use crate::scopes::Scope;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::swc::parser::Syntax;
use deno_ast::view::ProgramRef;
use deno_ast::ParsedSource;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Instant;

pub use deno_ast::view::SourceFile;

#[derive(Default)]
pub struct LinterBuilder {
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  syntax: deno_ast::swc::parser::Syntax,
  rules: Vec<Arc<dyn LintRule>>,
  plugins: Vec<Arc<dyn Plugin>>,
}

impl LinterBuilder {
  pub fn default() -> Self {
    Self {
      ignore_file_directive: "deno-lint-ignore-file".to_string(),
      ignore_diagnostic_directive: "deno-lint-ignore".to_string(),
      syntax: get_default_ts_config(),
      ..Default::default()
    }
  }

  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directive,
      self.ignore_diagnostic_directive,
      self.syntax,
      self.rules,
      self.plugins,
    )
  }

  pub fn ignore_file_directive(mut self, directive: &str) -> Self {
    self.ignore_file_directive = directive.to_owned();
    self
  }

  pub fn ignore_diagnostic_directive(mut self, directive: &str) -> Self {
    self.ignore_diagnostic_directive = directive.to_owned();
    self
  }

  pub fn syntax(mut self, syntax: Syntax) -> Self {
    self.syntax = syntax;
    self
  }

  pub fn rules(mut self, rules: Vec<Arc<dyn LintRule>>) -> Self {
    self.rules = rules;
    self
  }

  pub fn plugins(mut self, plugins: Vec<Arc<dyn Plugin>>) -> Self {
    self.plugins = plugins;
    self
  }
}

pub struct Linter {
  ast_parser: AstParser,
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  syntax: Syntax,
  rules: Vec<Arc<dyn LintRule>>,
  plugins: Vec<Arc<dyn Plugin>>,
}

impl Linter {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    syntax: Syntax,
    rules: Vec<Arc<dyn LintRule>>,
    plugins: Vec<Arc<dyn Plugin>>,
  ) -> Self {
    Linter {
      ast_parser: AstParser::new(),
      ignore_file_directive,
      ignore_diagnostic_directive,
      syntax,
      rules,
      plugins,
    }
  }

  pub fn lint(
    mut self,
    file_name: String,
    source_code: String,
  ) -> Result<(ParsedSource, Vec<LintDiagnostic>), SwcDiagnostic> {
    let start = Instant::now();

    let parse_result =
      self
        .ast_parser
        .parse_program(&file_name, self.syntax, source_code);
    let end_parse_program = Instant::now();
    debug!(
      "ast_parser.parse_program took {:#?}",
      end_parse_program - start
    );
    let parsed_source = parse_result?;
    let diagnostics = self.lint_program(&parsed_source);

    let end = Instant::now();
    debug!("Linter::lint took {:#?}", end - start);
    Ok((parsed_source, diagnostics))
  }

  pub fn lint_with_ast(
    mut self,
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();
    let diagnostics = self.lint_program(parsed_source);
    let end = Instant::now();
    debug!("Linter::lint_with_ast took {:#?}", end - start);

    diagnostics
  }

  fn filter_diagnostics(&self, mut context: Context) -> Vec<LintDiagnostic> {
    let start = Instant::now();

    let mut filtered_diagnostics = context.check_ignore_directive_usage();
    // Run `ban-unused-ignore`
    filtered_diagnostics.extend(context.ban_unused_ignore(&self.rules));
    // Run `ban-unknown-rule-code`
    filtered_diagnostics.extend(context.ban_unknown_rule_code());
    filtered_diagnostics.sort_by_key(|d| d.range.start.line_index);

    let end = Instant::now();
    debug!("Linter::filter_diagnostics took {:#?}", end - start);

    filtered_diagnostics
  }

  fn lint_program(
    &mut self,
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();

    let control_flow = ControlFlow::analyze(parsed_source.program_ref().into());
    let top_level_ctxt = deno_ast::swc::common::GLOBALS
      .set(&self.ast_parser.globals, || {
        SyntaxContext::empty().apply_mark(self.ast_parser.top_level_mark)
      });

    let diagnostics = parsed_source.with_view(|pg| {
      let file_ignore_directive =
        parse_file_ignore_directives(&self.ignore_file_directive, pg);

      // If a global ignore directive that has no codes specified exists, we must skip linting on
      // this file.
      if matches!(file_ignore_directive, Some(ref file_ignore) if file_ignore.ignore_all())
      {
        return vec![];
      }

      let line_ignore_directives =
        parse_line_ignore_directives(&self.ignore_diagnostic_directive, pg);

      let scope = Scope::analyze(pg);

      let mut context = Context::new(
        parsed_source.specifier().to_string(),
        parsed_source.source(),
        pg,
        file_ignore_directive,
        line_ignore_directives,
        scope,
        control_flow,
        top_level_ctxt,
      );

      // First sort lint rules by priority and alphabetically.
      let mut sorted_rules = self.rules.clone();
      sorted_rules.sort_by(|rule1, rule2| {
        let priority_cmp = rule1.priority().cmp(&rule2.priority());

        if priority_cmp == Ordering::Equal {
          return rule1.code().cmp(&rule2.code());
        }

        priority_cmp
      });

      // Run builtin rules
      for rule in sorted_rules.iter() {
        rule.lint_program_with_ast_view(&mut context, pg);
      }

      // TODO(bartlomieju): plugins rules should be sorted by priority as well.
      // Run plugin rules
      for plugin in self.plugins.iter() {
        // Ignore any error
        let _ = plugin.run(&mut context, parsed_source.program_ref().into());
      }

      self.filter_diagnostics(context)
    });

    let end = Instant::now();
    debug!("Linter::lint_module took {:#?}", end - start);

    diagnostics
  }
}

pub trait Plugin: std::fmt::Debug + Send + Sync {
  fn run(
    &self,
    context: &mut Context,
    program: ProgramRef,
  ) -> anyhow::Result<()>;
}
