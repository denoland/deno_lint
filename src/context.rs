// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::control_flow::ControlFlow;
use crate::diagnostic::{LintDiagnostic, Position, Range};
use crate::ignore_directives::{
  CodeStatus, FileIgnoreDirective, LineIgnoreDirective,
};
use crate::rules::{self, get_all_rules, LintRule};
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::{Span, SyntaxContext};
use deno_ast::{view as ast_view, SourceRange, RootNode, SourcePos};
use deno_ast::SourceTextInfo;
use deno_ast::MediaType;
use deno_ast::Scope;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

/// `Context` stores data needed while performing all lint rules to a file.
pub struct Context<'view> {
  /// File name on which the lint rule is run
  file_name: String,

  /// The media type which linter was configured with. Can be used
  /// to skip checking some rules.
  media_type: MediaType,

  /// Stores diagnostics that are generated while linting
  diagnostics: Vec<LintDiagnostic>,

  /// Information about the file text.
  text_info: &'view SourceTextInfo,

  /// The AST view of the program, which for example can be used for getting
  /// comments
  program: ast_view::Program<'view>,

  /// File-level ignore directive (`deno-lint-ignore-file`)
  file_ignore_directive: Option<FileIgnoreDirective>,

  /// The map that stores line-level ignore directives (`deno-lint-ignore`).
  /// The key of the map is line number.
  line_ignore_directives: HashMap<usize, LineIgnoreDirective>,

  /// Scope analysis result
  scope: Scope,

  /// Control-flow analysis result
  control_flow: ControlFlow,

  /// The `SyntaxContext` of the top level
  top_level_ctxt: SyntaxContext,

  /// The `SyntaxContext` of any unresolved identifiers
  unresolved_ctxt: SyntaxContext,

  /// A value to control whether the node's children will be traversed or not.
  traverse_flow: TraverseFlow,

  /// Whether to check unknown rules
  check_unknown_rules: bool,
}

impl<'view> Context<'view> {
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    file_name: String,
    media_type: MediaType,
    text_info: &'view SourceTextInfo,
    program: ast_view::Program<'view>,
    file_ignore_directive: Option<FileIgnoreDirective>,
    line_ignore_directives: HashMap<usize, LineIgnoreDirective>,
    scope: Scope,
    control_flow: ControlFlow,
    top_level_ctxt: SyntaxContext,
    unresolved_ctxt: SyntaxContext,
    check_unknown_rules: bool,
  ) -> Self {
    Self {
      file_name,
      media_type,
      text_info,
      program,
      file_ignore_directive,
      line_ignore_directives,
      scope,
      control_flow,
      top_level_ctxt,
      unresolved_ctxt,
      diagnostics: Vec::new(),
      traverse_flow: TraverseFlow::default(),
      check_unknown_rules,
    }
  }
  pub fn file_name(&self) -> &str {
    &self.file_name
  }

  pub fn media_type(&self) -> MediaType {
    self.media_type
  }

  pub fn diagnostics(&self) -> &[LintDiagnostic] {
    &self.diagnostics
  }

  pub fn text_info(&self) -> &SourceTextInfo {
    self.text_info
  }

  pub fn file_text_substring(&self, range: &SourceRange) -> &str {
    self.text_info.range_text(range)
  }

  pub fn program(&self) -> &ast_view::Program<'view> {
    &self.program
  }

  pub fn file_ignore_directive(&self) -> Option<&FileIgnoreDirective> {
    self.file_ignore_directive.as_ref()
  }

  pub fn line_ignore_directives(&self) -> &HashMap<usize, LineIgnoreDirective> {
    &self.line_ignore_directives
  }

  pub fn scope(&self) -> &Scope {
    &self.scope
  }

  pub fn control_flow(&self) -> &ControlFlow {
    &self.control_flow
  }

  pub(crate) fn top_level_ctxt(&self) -> SyntaxContext {
    self.top_level_ctxt
  }

  pub(crate) fn unresolved_ctxt(&self) -> SyntaxContext {
    self.unresolved_ctxt
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

  pub fn all_comments(&self) -> impl Iterator<Item = &'view Comment> {
    self.program.comment_container().all_comments()
  }

  pub fn leading_comments_at(
    &self,
    start: SourcePos,
  ) -> impl Iterator<Item = &'view Comment> {
    self
      .program
      .comment_container()
      .leading_comments(start)
  }

  pub fn trailing_comments_at(
    &self,
    end: SourcePos,
  ) -> impl Iterator<Item = &'view Comment> {
    self
      .program
      .comment_container()
      .trailing_comments(end)
  }

  /// Mark ignore directives as used if that directive actually suppresses some
  /// diagnostic, and return a list of diagnostics that are not ignored.
  /// Make sure that this method is called after all lint rules have been
  /// executed.
  pub(crate) fn check_ignore_directive_usage(&mut self) -> Vec<LintDiagnostic> {
    let mut filtered = Vec::new();

    for diagnostic in self.diagnostics.iter().cloned() {
      if let Some(f) = self.file_ignore_directive.as_mut() {
        if f.check_used(&diagnostic.code) {
          continue;
        }
      }

      let diagnostic_line = diagnostic.range.start.line_index;
      if diagnostic_line > 0 {
        if let Some(l) =
          self.line_ignore_directives.get_mut(&(diagnostic_line - 1))
        {
          if l.check_used(&diagnostic.code) {
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
    specified_rules: &[Arc<dyn LintRule>],
  ) -> Vec<LintDiagnostic> {
    const CODE: &str = "ban-unused-ignore";

    // If there's a file-level ignore directive containing `ban-unused-ignore`,
    // exit without running this rule.
    if self
      .file_ignore_directive
      .as_ref()
      .map_or(false, |file_ignore| file_ignore.has_code(CODE))
    {
      return vec![];
    }

    let executed_builtin_codes: HashSet<&'static str> =
      specified_rules.iter().map(|r| r.code()).collect();
    let is_unused_code = |&(code, status): &(&String, &CodeStatus)| {
      let is_unknown = !executed_builtin_codes.contains(code.as_str());
      !status.used && !is_unknown
    };

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for (unused_code, _status) in
        file_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          file_ignore.range(),
          CODE,
          format!("Ignore for code \"{}\" was not used.", unused_code),
          None,
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
          line_ignore.range(),
          CODE,
          format!("Ignore for code \"{}\" was not used.", unused_code),
          None,
        );
        diagnostics.push(d);
      }
    }

    diagnostics
  }

  /// Lint rule implementation for `ban-unknown-rule-code`.
  /// This should be run after all normal rules.
  pub(crate) fn ban_unknown_rule_code(&mut self) -> Vec<LintDiagnostic> {
    let builtin_all_rule_codes: HashSet<&'static str> =
      get_all_rules().iter().map(|r| r.code()).collect();
    let is_unknown_rule =
      |code: &&String| !builtin_all_rule_codes.contains(code.as_str());

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for unknown_rule_code in
        file_ignore.codes().keys().filter(is_unknown_rule)
      {
        let d = self.create_diagnostic(
          file_ignore.range(),
          rules::ban_unknown_rule_code::CODE,
          format!("Unknown rule for code \"{}\"", unknown_rule_code),
          None,
        );
        diagnostics.push(d);
      }
    }

    for line_ignore in self.line_ignore_directives.values() {
      for unknown_rule_code in
        line_ignore.codes().keys().filter(is_unknown_rule)
      {
        let d = self.create_diagnostic(
          line_ignore.range(),
          rules::ban_unknown_rule_code::CODE,
          format!("Unknown rule for code \"{}\"", unknown_rule_code),
          None,
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
    let diagnostic =
      self.create_diagnostic(range, code.to_string(), message.to_string(), None);
    self.diagnostics.push(diagnostic);
  }

  pub fn add_diagnostic_with_hint(
    &mut self,
    range: SourceRange,
    code: impl ToString,
    message: impl ToString,
    hint: impl ToString,
  ) {
    let diagnostic =
      self.create_diagnostic(range, code, message, Some(hint.to_string()));
    self.diagnostics.push(diagnostic);
  }

  pub(crate) fn create_diagnostic(
    &self,
    range: SourceRange,
    code: impl ToString,
    message: impl ToString,
    maybe_hint: Option<String>,
  ) -> LintDiagnostic {
    let time_start = Instant::now();
    let start = Position::new(
      range.start - self.text_info.range().start,
      self.text_info.line_and_column_index(range.start),
    );
    let end = Position::new(
      range.end - self.text_info.range().start,
      self.text_info.line_and_column_index(range.end),
    );

    let diagnostic = LintDiagnostic {
      range: Range { start, end },
      filename: self.file_name.clone(),
      message: message.to_string(),
      code: code.to_string(),
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
