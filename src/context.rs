// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::control_flow::ControlFlow;
use crate::diagnostic::{
  LintDiagnostic, LintDiagnosticDetails, LintDiagnosticRange, LintDocsUrl,
  LintFix,
};
use crate::ignore_directives::{
  parse_line_ignore_directives, CodeStatus, FileIgnoreDirective,
  LineIgnoreDirective,
};
use crate::linter::LinterContext;
use crate::rules;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::util::take::Take;
use deno_ast::swc::common::{SourceMap, SyntaxContext};
use deno_ast::SourceTextInfo;
use deno_ast::{
  view as ast_view, ParsedSource, RootNode, SourcePos, SourceRange,
};
use deno_ast::{MediaType, ModuleSpecifier};
use deno_ast::{MultiThreadedComments, Scope};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// `Context` stores all data needed to perform linting of a particular file.
pub struct Context<'a> {
  parsed_source: ParsedSource,
  diagnostics: Vec<LintDiagnostic>,
  program: ast_view::Program<'a>,
  file_ignore_directive: Option<FileIgnoreDirective>,
  line_ignore_directives: HashMap<usize, LineIgnoreDirective>,
  scope: Scope,
  control_flow: ControlFlow,
  traverse_flow: TraverseFlow,
  check_unknown_rules: bool,
  #[allow(clippy::redundant_allocation)] // This type comes from SWC.
  jsx_factory: Option<Rc<Box<Expr>>>,
  #[allow(clippy::redundant_allocation)] // This type comes from SWC.
  jsx_fragment_factory: Option<Rc<Box<Expr>>>,
}

impl<'a> Context<'a> {
  pub(crate) fn new(
    linter_ctx: &'a LinterContext,
    parsed_source: ParsedSource,
    program: ast_view::Program<'a>,
    file_ignore_directive: Option<FileIgnoreDirective>,
    default_jsx_factory: Option<String>,
    default_jsx_fragment_factory: Option<String>,
  ) -> Self {
    let line_ignore_directives = parse_line_ignore_directives(
      linter_ctx.ignore_diagnostic_directive,
      program,
    );
    let scope = Scope::analyze(program);
    let control_flow =
      ControlFlow::analyze(program, parsed_source.unresolved_context());

    let mut jsx_factory = None;
    let mut jsx_fragment_factory = None;

    parsed_source.globals().with(|marks| {
      let top_level_mark = marks.top_level;

      if let Some(leading_comments) = parsed_source.get_leading_comments() {
        let jsx_directives =
          deno_ast::swc::transforms::react::JsxDirectives::from_comments(
            &SourceMap::default(),
            #[allow(clippy::disallowed_types)]
            deno_ast::swc::common::Span::dummy(),
            leading_comments,
            top_level_mark,
          );

        jsx_factory = jsx_directives.pragma;
        jsx_fragment_factory = jsx_directives.pragma_frag;
      }

      if jsx_factory.is_none() {
        if let Some(factory) = default_jsx_factory {
          jsx_factory = Some(Rc::new(
            deno_ast::swc::transforms::react::parse_expr_for_jsx(
              &SourceMap::default(),
              "jsx",
              Rc::new(factory),
              top_level_mark,
            ),
          ));
        }
      }
      if jsx_fragment_factory.is_none() {
        if let Some(factory) = default_jsx_fragment_factory {
          jsx_fragment_factory = Some(Rc::new(
            deno_ast::swc::transforms::react::parse_expr_for_jsx(
              &SourceMap::default(),
              "jsxFragment",
              Rc::new(factory),
              top_level_mark,
            ),
          ));
        }
      }
    });

    Self {
      file_ignore_directive,
      line_ignore_directives,
      scope,
      control_flow,
      program,
      parsed_source,
      diagnostics: Vec::new(),
      traverse_flow: TraverseFlow::default(),
      check_unknown_rules: linter_ctx.check_unknown_rules,
      jsx_factory,
      jsx_fragment_factory,
    }
  }

  /// File specifier on which the lint rule is run.
  pub fn specifier(&self) -> &ModuleSpecifier {
    self.parsed_source.specifier()
  }

  /// The media type which linter was configured with. Can be used
  /// to skip checking some rules.
  pub fn media_type(&self) -> MediaType {
    self.parsed_source.media_type()
  }

  /// Comment collection.
  pub fn comments(&self) -> &MultiThreadedComments {
    self.parsed_source.comments()
  }

  /// Stores diagnostics that are generated while linting
  pub fn diagnostics(&self) -> &[LintDiagnostic] {
    &self.diagnostics
  }

  /// Parsed source of the program.
  pub fn parsed_source(&self) -> &ParsedSource {
    &self.parsed_source
  }

  /// Information about the file text.
  pub fn text_info(&self) -> &SourceTextInfo {
    self.parsed_source.text_info_lazy()
  }

  /// The AST view of the program, which for example can be used for getting
  /// comments
  pub fn program(&self) -> ast_view::Program<'a> {
    self.program
  }

  /// File-level ignore directive (`deno-lint-ignore-file`)
  pub fn file_ignore_directive(&self) -> Option<&FileIgnoreDirective> {
    self.file_ignore_directive.as_ref()
  }

  /// The map that stores line-level ignore directives (`deno-lint-ignore`).
  /// The key of the map is line number.
  pub fn line_ignore_directives(&self) -> &HashMap<usize, LineIgnoreDirective> {
    &self.line_ignore_directives
  }

  /// Scope analysis result
  pub fn scope(&self) -> &Scope {
    &self.scope
  }

  /// Control-flow analysis result
  pub fn control_flow(&self) -> &ControlFlow {
    &self.control_flow
  }

  /// Get the JSX factory expression for this file, if one is specified (via
  /// pragma or using a default). If this file is not JSX, uses the automatic
  /// transform, or the default factory is not specified, this will return
  /// `None`.
  pub fn jsx_factory(&self) -> Option<Rc<Box<Expr>>> {
    self.jsx_factory.clone()
  }

  /// Get the JSX fragment factory expression for this file, if one is specified
  /// (via pragma or using a default). If this file is not JSX, uses the
  /// automatic transform, or the default factory is not specified, this will
  /// return `None`.
  pub fn jsx_fragment_factory(&self) -> Option<Rc<Box<Expr>>> {
    self.jsx_fragment_factory.clone()
  }

  /// The `SyntaxContext` of any unresolved identifiers
  pub(crate) fn unresolved_ctxt(&self) -> SyntaxContext {
    self.parsed_source.unresolved_context()
  }

  pub(crate) fn assert_traverse_init(&self) {
    self.traverse_flow.assert_init();
  }

  pub(crate) fn should_stop_traverse(&mut self) -> bool {
    self.traverse_flow.should_stop()
  }

  pub(crate) fn stop_traverse(&mut self) {
    self.traverse_flow.set_stop_traverse();
  }

  pub fn all_comments(&self) -> impl Iterator<Item = &'a Comment> {
    self.program.comment_container().all_comments()
  }

  pub fn leading_comments_at(
    &self,
    start: SourcePos,
  ) -> impl Iterator<Item = &'a Comment> {
    self.program.comment_container().leading_comments(start)
  }

  pub fn trailing_comments_at(
    &self,
    end: SourcePos,
  ) -> impl Iterator<Item = &'a Comment> {
    self.program.comment_container().trailing_comments(end)
  }

  /// Mark ignore directives as used if that directive actually suppresses some
  /// diagnostic, and return a list of diagnostics that are not ignored.
  /// Make sure that this method is called after all lint rules have been
  /// executed.
  pub(crate) fn check_ignore_directive_usage(&mut self) -> Vec<LintDiagnostic> {
    let mut filtered = Vec::new();

    for diagnostic in self.diagnostics.iter().cloned() {
      if let Some(f) = self.file_ignore_directive.as_mut() {
        if f.check_used(&diagnostic.details.code) {
          continue;
        }
      }
      let Some(range) = diagnostic.range.as_ref() else {
        continue;
      };

      let diagnostic_line = range.text_info.line_index(range.range.start);
      if diagnostic_line > 0 {
        if let Some(l) =
          self.line_ignore_directives.get_mut(&(diagnostic_line - 1))
        {
          if l.check_used(&diagnostic.details.code) {
            continue;
          }
        }
      }

      filtered.push(diagnostic);
    }

    filtered
  }

  /// Lint rule implementation for `ban-unused-ignore`.
  /// This should be run after all normal rules have been finished because this
  /// works for diagnostics reported by other rules.
  pub(crate) fn ban_unused_ignore(
    &self,
    known_rules_codes: &HashSet<Cow<'static, str>>,
  ) -> Vec<LintDiagnostic> {
    const CODE: &str = "ban-unused-ignore";

    // If there's a file-level ignore directive containing `ban-unused-ignore`,
    // exit without running this rule.
    if self
      .file_ignore_directive
      .as_ref()
      .is_some_and(|file_ignore| file_ignore.has_code(CODE))
    {
      return vec![];
    }

    let is_unused_code = |&(code, status): &(&String, &CodeStatus)| {
      let is_unknown = !known_rules_codes.contains(code.as_str());
      !status.used && !is_unknown
    };

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for (unused_code, _status) in
        file_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(file_ignore.range())),
          self.create_diagnostic_details(
            CODE,
            format!("Ignore for code \"{}\" was not used.", unused_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    for line_ignore in self.line_ignore_directives.values() {
      // We do nothing special even if the line-level ignore directive contains
      // `ban-unused-ignore`. `ban-unused-ignore` can be ignored only via the
      // file-level directive.

      for (unused_code, _status) in
        line_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(line_ignore.range())),
          self.create_diagnostic_details(
            CODE,
            format!("Ignore for code \"{}\" was not used.", unused_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    diagnostics
  }

  // TODO(bartlomieju): this should be a regular lint rule, not a mathod on this
  // struct.
  /// Lint rule implementation for `ban-unknown-rule-code`.
  /// This should be run after all normal rules.
  pub(crate) fn ban_unknown_rule_code(
    &mut self,
    enabled_rules: &HashSet<Cow<'static, str>>,
  ) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for unknown_rule_code in file_ignore
        .codes()
        .keys()
        .filter(|code| !enabled_rules.contains(code.as_str()))
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(file_ignore.range())),
          self.create_diagnostic_details(
            rules::ban_unknown_rule_code::CODE,
            format!("Unknown rule for code \"{}\"", unknown_rule_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    for line_ignore in self.line_ignore_directives.values() {
      for unknown_rule_code in line_ignore
        .codes()
        .keys()
        .filter(|code| !enabled_rules.contains(code.as_str()))
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(line_ignore.range())),
          self.create_diagnostic_details(
            rules::ban_unknown_rule_code::CODE,
            format!("Unknown rule for code \"{}\"", unknown_rule_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    if !diagnostics.is_empty() {
      if let Some(f) = self.file_ignore_directive.as_mut() {
        f.check_used(rules::ban_unknown_rule_code::CODE);
      }
    }

    if self.check_unknown_rules
      && !self
        .file_ignore_directive()
        .map(|f| f.has_code(rules::ban_unknown_rule_code::CODE))
        .unwrap_or(false)
    {
      diagnostics
    } else {
      vec![]
    }
  }

  pub fn add_diagnostic(
    &mut self,
    range: SourceRange,
    code: impl ToString,
    message: impl ToString,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(range)),
      self.create_diagnostic_details(
        code,
        message.to_string(),
        None,
        Vec::new(),
      ),
    );
  }

  pub fn add_diagnostic_with_hint(
    &mut self,
    range: SourceRange,
    code: impl ToString,
    message: impl ToString,
    hint: impl ToString,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(range)),
      self.create_diagnostic_details(
        code,
        message,
        Some(hint.to_string()),
        Vec::new(),
      ),
    );
  }

  pub fn add_diagnostic_with_fixes(
    &mut self,
    range: SourceRange,
    code: impl ToString,
    message: impl ToString,
    hint: Option<String>,
    fixes: Vec<LintFix>,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(range)),
      self.create_diagnostic_details(code, message, hint, fixes),
    );
  }

  pub fn add_diagnostic_details(
    &mut self,
    maybe_range: Option<LintDiagnosticRange>,
    details: LintDiagnosticDetails,
  ) {
    self
      .diagnostics
      .push(self.create_diagnostic(maybe_range, details));
  }

  /// Add fully constructed diagnostics.
  ///
  /// This function can be used by the "external linter" to provide its own
  /// diagnostics.
  pub fn add_external_diagnostics(&mut self, diagnostics: &[LintDiagnostic]) {
    self.diagnostics.extend_from_slice(diagnostics);
  }

  pub(crate) fn create_diagnostic(
    &self,
    maybe_range: Option<LintDiagnosticRange>,
    details: LintDiagnosticDetails,
  ) -> LintDiagnostic {
    LintDiagnostic {
      specifier: self.specifier().clone(),
      range: maybe_range,
      details,
    }
  }

  pub(crate) fn create_diagnostic_details(
    &self,
    code: impl ToString,
    message: impl ToString,
    maybe_hint: Option<String>,
    fixes: Vec<LintFix>,
  ) -> LintDiagnosticDetails {
    LintDiagnosticDetails {
      message: message.to_string(),
      code: code.to_string(),
      hint: maybe_hint,
      fixes,
      custom_docs_url: LintDocsUrl::Default,
      info: vec![],
    }
  }

  pub(crate) fn create_diagnostic_range(
    &self,
    range: SourceRange,
  ) -> LintDiagnosticRange {
    LintDiagnosticRange {
      range,
      text_info: self.text_info().clone(),
      description: None,
    }
  }
}

/// A struct containing a boolean value to control whether a node's children
/// will be traversed or not.
/// If there's no need to further traverse children nodes, you can call
/// `ctx.stop_traverse()` from inside a handler method, which will cancel
/// further traverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TraverseFlow {
  stop_traverse: bool,
}

impl TraverseFlow {
  fn set_stop_traverse(&mut self) {
    self.stop_traverse = true;
  }

  fn reset(&mut self) {
    self.stop_traverse = false;
  }

  fn assert_init(&self) {
    assert!(!self.stop_traverse);
  }

  fn should_stop(&mut self) -> bool {
    let stop = self.stop_traverse;
    self.reset();
    stop
  }
}
