// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::ast_parser::get_default_ts_config;
use crate::ast_parser::AstParser;
use crate::ast_parser::SwcDiagnosticBuffer;
use crate::control_flow::ControlFlow;
use crate::diagnostic::{LintDiagnostic, Position, Range};
use crate::ignore_directives::parse_ignore_comment;
use crate::ignore_directives::parse_ignore_directives;
use crate::ignore_directives::IgnoreDirective;
use crate::rules::LintRule;
use crate::scopes::analyze;
use crate::scopes::Scope;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use swc_common::comments::SingleThreadedComments;
use swc_common::BytePos;
use swc_common::SourceMap;
use swc_common::Span;
use swc_common::Spanned;
use swc_common::{comments::Comment, SyntaxContext};
use swc_ecmascript::parser::Syntax;

pub struct Context {
  pub file_name: String,
  pub diagnostics: Vec<LintDiagnostic>,
  pub source_map: Rc<SourceMap>,
  pub(crate) leading_comments: HashMap<BytePos, Vec<Comment>>,
  pub(crate) trailing_comments: HashMap<BytePos, Vec<Comment>>,
  pub ignore_directives: RefCell<Vec<IgnoreDirective>>,
  pub(crate) scope: Scope,
  pub(crate) control_flow: ControlFlow,
  pub(crate) top_level_ctxt: SyntaxContext,
}

impl Context {
  pub(crate) fn add_diagnostic(
    &mut self,
    span: Span,
    code: impl Into<String>,
    message: impl Into<String>,
  ) {
    let diagnostic = self.create_diagnostic(span, code, message, None);
    self.diagnostics.push(diagnostic);
  }

  pub(crate) fn add_diagnostic_with_hint(
    &mut self,
    span: Span,
    code: impl Into<String>,
    message: impl Into<String>,
    hint: impl Into<String>,
  ) {
    let diagnostic =
      self.create_diagnostic(span, code, message, Some(hint.into()));
    self.diagnostics.push(diagnostic);
  }

  fn create_diagnostic(
    &self,
    span: Span,
    code: impl Into<String>,
    message: impl Into<String>,
    maybe_hint: Option<String>,
  ) -> LintDiagnostic {
    let time_start = Instant::now();
    let start = Position::new(
      self.source_map.lookup_byte_offset(span.lo()).pos,
      self.source_map.lookup_char_pos(span.lo()),
    );
    let end = Position::new(
      self.source_map.lookup_byte_offset(span.hi()).pos,
      self.source_map.lookup_char_pos(span.hi()),
    );

    let diagnostic = LintDiagnostic {
      range: Range { start, end },
      filename: self.file_name.clone(),
      message: message.into(),
      code: code.into(),
      hint: maybe_hint,
    };

    let time_end = Instant::now();
    debug!(
      "Context::create_diagnostic took {:?}",
      time_end - time_start
    );
    diagnostic
  }
}

pub struct LinterBuilder {
  ignore_file_directives: Vec<String>,
  ignore_diagnostic_directives: Vec<String>,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: swc_ecmascript::parser::Syntax,
  rules: Vec<Box<dyn LintRule>>,
}

impl LinterBuilder {
  pub fn default() -> Self {
    Self {
      ignore_file_directives: vec!["deno-lint-ignore-file".to_string()],
      ignore_diagnostic_directives: vec!["deno-lint-ignore".to_string()],
      lint_unused_ignore_directives: true,
      lint_unknown_rules: true,
      syntax: get_default_ts_config(),
      rules: vec![],
    }
  }

  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directives,
      self.ignore_diagnostic_directives,
      self.lint_unused_ignore_directives,
      self.lint_unknown_rules,
      self.syntax,
      self.rules,
    )
  }

  pub fn ignore_file_directives(mut self, directives: Vec<&str>) -> Self {
    self.ignore_file_directives =
      directives.iter().map(|s| s.to_string()).collect();
    self
  }

  pub fn ignore_diagnostic_directives(mut self, directives: Vec<&str>) -> Self {
    self.ignore_diagnostic_directives =
      directives.iter().map(|s| s.to_string()).collect();
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
}

pub struct Linter {
  has_linted: bool,
  ast_parser: AstParser,
  ignore_file_directives: Vec<String>,
  ignore_diagnostic_directives: Vec<String>,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: Syntax,
  rules: Vec<Box<dyn LintRule>>,
}

impl Linter {
  fn new(
    ignore_file_directives: Vec<String>,
    ignore_diagnostic_directives: Vec<String>,
    lint_unused_ignore_directives: bool,
    lint_unknown_rules: bool,
    syntax: Syntax,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Self {
    Linter {
      has_linted: false,
      ast_parser: AstParser::new(),
      ignore_file_directives,
      ignore_diagnostic_directives,
      lint_unused_ignore_directives,
      lint_unknown_rules,
      syntax,
      rules,
    }
  }

  pub fn lint(
    &mut self,
    file_name: String,
    source_code: String,
  ) -> Result<Vec<LintDiagnostic>, SwcDiagnosticBuffer> {
    assert!(
      !self.has_linted,
      "Linter can be used only on a single module."
    );
    self.has_linted = true;
    let start = Instant::now();
    let mut diagnostics = vec![];

    if !source_code.is_empty() {
      let (parse_result, comments) =
        self
          .ast_parser
          .parse_program(&file_name, self.syntax, &source_code);
      let end_parse_program = Instant::now();
      debug!(
        "ast_parser.parse_program took {:#?}",
        end_parse_program - start
      );
      let program = parse_result?;
      diagnostics = self.lint_program(file_name, program, comments);
    }

    let end = Instant::now();
    debug!("Linter::lint took {:#?}", end - start);
    Ok(diagnostics)
  }

  fn filter_diagnostics(
    &self,
    context: &mut Context,
    rules: &[Box<dyn LintRule>],
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();
    let ignore_directives = context.ignore_directives.clone();
    let diagnostics = &context.diagnostics;

    let rule_codes = rules
      .iter()
      .map(|r| r.code().to_string())
      .collect::<Vec<String>>();

    let mut filtered_diagnostics: Vec<LintDiagnostic> = diagnostics
      .as_slice()
      .iter()
      .cloned()
      .filter(|diagnostic| {
        !ignore_directives
          .borrow_mut()
          .iter_mut()
          .any(|ignore_directive| {
            ignore_directive.maybe_ignore_diagnostic(&diagnostic)
          })
      })
      .collect();

    if self.lint_unused_ignore_directives || self.lint_unknown_rules {
      for ignore_directive in ignore_directives.borrow().iter() {
        for (code, used) in ignore_directive.used_codes.iter() {
          if self.lint_unused_ignore_directives
            && !used
            && rule_codes.contains(code)
          {
            let diagnostic = context.create_diagnostic(
              ignore_directive.span,
              "ban-unused-ignore",
              format!("Ignore for code \"{}\" was not used.", code),
              None,
            );
            filtered_diagnostics.push(diagnostic);
          }

          if self.lint_unknown_rules && !rule_codes.contains(code) {
            filtered_diagnostics.push(context.create_diagnostic(
              ignore_directive.span,
              "ban-unknown-rule-code",
              format!("Unknown rule for code \"{}\"", code),
              None,
            ))
          }
        }
      }
    }

    filtered_diagnostics
      .sort_by(|a, b| a.range.start.line.cmp(&b.range.start.line));

    let end = Instant::now();
    debug!("Linter::filter_diagnostics took {:#?}", end - start);

    filtered_diagnostics
  }

  fn lint_program(
    &self,
    file_name: String,
    program: swc_ecmascript::ast::Program,
    comments: SingleThreadedComments,
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();
    let file_ignore_directive =
      comments.with_leading(program.span().lo(), |c| {
        c.iter().find_map(|comment| {
          parse_ignore_comment(
            &self.ignore_file_directives,
            &*self.ast_parser.source_map,
            comment,
            true,
          )
        })
      });

    // If there's a file ignore directive that has no codes specified we must ignore
    // whole file and skip linting it.
    if let Some(ignore_directive) = &file_ignore_directive {
      if ignore_directive.codes.is_empty() {
        return vec![];
      }
    }

    let (leading, trailing) = comments.take_all();
    let leading_coms = Rc::try_unwrap(leading)
      .expect("Failed to get leading comments")
      .into_inner();
    let leading = leading_coms.into_iter().collect();
    let trailing_coms = Rc::try_unwrap(trailing)
      .expect("Failed to get leading comments")
      .into_inner();
    let trailing = trailing_coms.into_iter().collect();

    let mut ignore_directives = parse_ignore_directives(
      &self.ignore_diagnostic_directives,
      &self.ast_parser.source_map,
      &leading,
      &trailing,
    );

    if let Some(ignore_directive) = file_ignore_directive {
      ignore_directives.insert(0, ignore_directive);
    }

    let scope = analyze(&program);
    let control_flow = ControlFlow::analyze(&program);
    let top_level_ctxt = swc_common::GLOBALS
      .set(&self.ast_parser.globals, || {
        SyntaxContext::empty().apply_mark(self.ast_parser.top_level_mark)
      });

    let mut context = Context {
      file_name,
      diagnostics: vec![],
      source_map: self.ast_parser.source_map.clone(),
      leading_comments: leading,
      trailing_comments: trailing,
      ignore_directives: RefCell::new(ignore_directives),
      scope,
      control_flow,
      top_level_ctxt,
    };

    for rule in &self.rules {
      rule.lint_program(&mut context, &program);
    }

    let d = self.filter_diagnostics(&mut context, &self.rules);
    let end = Instant::now();
    debug!("Linter::lint_module took {:#?}", end - start);

    d
  }
}
