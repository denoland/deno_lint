// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::ast_parser::get_default_ts_config;
use crate::ast_parser::AstParser;
use crate::ast_parser::ParsedData;
use crate::ast_parser::SwcDiagnosticBuffer;
use crate::context::Context;
use crate::control_flow::ControlFlow;
use crate::diagnostic::LintDiagnostic;
use crate::ignore_directives::{
  parse_file_ignore_directives, parse_line_ignore_directives,
};
use crate::rules::LintRule;
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view::{self as AstView};
use std::rc::Rc;
use std::time::Instant;
use swc_common::comments::SingleThreadedComments;
use swc_common::SourceMap;
use swc_common::SyntaxContext;
use swc_ecmascript::parser::token::TokenAndSpan;
use swc_ecmascript::parser::Syntax;

pub use swc_common::SourceFile;

pub struct LinterBuilder {
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: swc_ecmascript::parser::Syntax,
  rules: Vec<Box<dyn LintRule>>,
  plugins: Vec<Box<dyn Plugin>>,
}

impl LinterBuilder {
  pub fn default() -> Self {
    Self {
      ignore_file_directive: "deno-lint-ignore-file".to_string(),
      ignore_diagnostic_directive: "deno-lint-ignore".to_string(),
      lint_unused_ignore_directives: true,
      lint_unknown_rules: true,
      syntax: get_default_ts_config(),
      rules: vec![],
      plugins: vec![],
    }
  }

  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directive,
      self.ignore_diagnostic_directive,
      self.lint_unused_ignore_directives,
      self.lint_unknown_rules,
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

  pub fn lint_unused_ignore_directives(
    mut self,
    lint_unused_ignore_directives: bool,
  ) -> Self {
    self.lint_unused_ignore_directives = lint_unused_ignore_directives;
    self
  }

  pub fn lint_unknown_rules(mut self, lint_unknown_rules: bool) -> Self {
    self.lint_unknown_rules = lint_unknown_rules;
    self
  }

  pub fn syntax(mut self, syntax: Syntax) -> Self {
    self.syntax = syntax;
    self
  }

  pub fn rules(mut self, rules: Vec<Box<dyn LintRule>>) -> Self {
    self.rules = rules;
    self
  }

  pub fn add_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
    self.plugins.push(plugin);
    self
  }
}

pub struct Linter {
  ast_parser: AstParser,
  ignore_file_directive: String,
  ignore_diagnostic_directive: String,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: Syntax,
  rules: Vec<Box<dyn LintRule>>,
  plugins: Vec<Box<dyn Plugin>>,
}

impl Linter {
  fn new(
    ignore_file_directive: String,
    ignore_diagnostic_directive: String,
    lint_unused_ignore_directives: bool,
    lint_unknown_rules: bool,
    syntax: Syntax,
    rules: Vec<Box<dyn LintRule>>,
    plugins: Vec<Box<dyn Plugin>>,
  ) -> Self {
    Linter {
      ast_parser: AstParser::new(),
      ignore_file_directive,
      ignore_diagnostic_directive,
      lint_unused_ignore_directives,
      lint_unknown_rules,
      syntax,
      rules,
      plugins,
    }
  }

  pub fn lint(
    mut self,
    file_name: String,
    source_code: String,
  ) -> Result<
    (Rc<swc_common::SourceFile>, Vec<LintDiagnostic>),
    SwcDiagnosticBuffer,
  > {
    let start = Instant::now();

    let parse_result =
      self
        .ast_parser
        .parse_program(&file_name, self.syntax, &source_code);
    let end_parse_program = Instant::now();
    debug!(
      "ast_parser.parse_program took {:#?}",
      end_parse_program - start
    );
    let ParsedData {
      program,
      comments,
      tokens,
    } = parse_result?;
    let source_file = self
      .ast_parser
      .source_map
      .get_source_file(&swc_common::FileName::Custom(file_name.clone()))
      .unwrap();

    let diagnostics =
      self.lint_program(file_name, &program, &comments, &tokens, &source_file);

    let end = Instant::now();
    debug!("Linter::lint took {:#?}", end - start);
    Ok((source_file, diagnostics))
  }

  pub fn lint_with_ast(
    mut self,
    file_name: String,
    ast: &swc_ecmascript::ast::Program,
    comments: &SingleThreadedComments,
    source_map: Rc<SourceMap>,
    tokens: &[TokenAndSpan],
  ) -> Result<
    (Rc<swc_common::SourceFile>, Vec<LintDiagnostic>),
    SwcDiagnosticBuffer,
  > {
    let start = Instant::now();

    self.ast_parser.set_source_map(source_map);
    let source_file = self
      .ast_parser
      .source_map
      .get_source_file(&swc_common::FileName::Custom(file_name.clone()))
      .unwrap();

    let diagnostics =
      self.lint_program(file_name, ast, comments, tokens, &source_file);

    let end = Instant::now();
    debug!("Linter::lint_with_ast took {:#?}", end - start);

    Ok((source_file, diagnostics))
  }

  fn filter_diagnostics(&self, mut context: Context) -> Vec<LintDiagnostic> {
    let start = Instant::now();

    let mut filtered_diagnostics = context.check_ignore_directive_usage();
    if self.lint_unused_ignore_directives {
      filtered_diagnostics.extend(context.ban_unused_ignore(&self.rules));
    }
    if self.lint_unknown_rules {
      filtered_diagnostics.extend(context.ban_unknown_rule_code());
    }
    filtered_diagnostics.sort_by_key(|d| d.range.start.line);

    let end = Instant::now();
    debug!("Linter::filter_diagnostics took {:#?}", end - start);

    filtered_diagnostics
  }

  fn lint_program(
    &mut self,
    file_name: String,
    program: &swc_ecmascript::ast::Program,
    comments: &SingleThreadedComments,
    tokens: &[TokenAndSpan],
    source_file: &SourceFile,
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();

    let control_flow = ControlFlow::analyze(&program);
    let top_level_ctxt = swc_common::GLOBALS
      .set(&self.ast_parser.globals, || {
        SyntaxContext::empty().apply_mark(self.ast_parser.top_level_mark)
      });

    let program_info = AstView::ProgramInfo {
      program,
      source_file: Some(source_file),
      tokens: Some(tokens),
      comments: Some(comments),
    };

    let diagnostics = AstView::with_ast_view(program_info, |pg| {
      let file_ignore_directive = parse_file_ignore_directives(
        &self.ignore_file_directive,
        &*self.ast_parser.source_map,
        pg,
      );

      // If a global ignore directive that has no codes specified exists, we must skip linting on
      // this file.
      if matches!(file_ignore_directive, Some(ref file_ignore) if file_ignore.ignore_all())
      {
        return vec![];
      }

      let line_ignore_directives = parse_line_ignore_directives(
        &self.ignore_diagnostic_directive,
        &self.ast_parser.source_map,
        pg,
      );

      let scope = Scope::analyze(pg);

      let mut context = Context::new(
        file_name,
        Rc::clone(&self.ast_parser.source_map),
        pg,
        file_ignore_directive,
        line_ignore_directives,
        scope,
        control_flow,
        top_level_ctxt,
      );

      // Run builtin rules
      for rule in &self.rules {
        rule.lint_program_with_ast_view(&mut context, pg);
      }

      // Run plugin rules
      for plugin in self.plugins.iter_mut() {
        // Ignore any error
        let _ = plugin.run(&mut context, program.clone());
      }

      self.filter_diagnostics(context)
    });

    let end = Instant::now();
    debug!("Linter::lint_module took {:#?}", end - start);

    diagnostics
  }
}

pub trait Plugin {
  fn run(
    &mut self,
    context: &mut Context,
    program: swc_ecmascript::ast::Program,
  ) -> anyhow::Result<()>;
}
