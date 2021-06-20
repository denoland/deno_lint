use crate::control_flow::ControlFlow;
use crate::diagnostic::{LintDiagnostic, Position, Range};
use crate::ignore_directives::{
  CodeStatus, FileIgnoreDirective, LineIgnoreDirective,
};
use crate::rules::{get_all_rules, LintRule};
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view::{self as AstView, BytePos, RootNode};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;
use swc_common::comments::Comment;
use swc_common::{SourceMap, Span, SyntaxContext};

pub struct Context<'view> {
  file_name: String,
  diagnostics: Vec<LintDiagnostic>,
  plugin_codes: HashSet<String>,
  source_map: Rc<SourceMap>,
  program: AstView::Program<'view>,
  file_ignore_directive: Option<FileIgnoreDirective>,
  /// key is line number
  line_ignore_directives: HashMap<usize, LineIgnoreDirective>,
  scope: Scope,
  control_flow: ControlFlow,
  top_level_ctxt: SyntaxContext,
}

impl<'view> Context<'view> {
  pub(crate) fn new(
    file_name: String,
    source_map: Rc<SourceMap>,
    program: AstView::Program<'view>,
    file_ignore_directive: Option<FileIgnoreDirective>,
    line_ignore_directives: HashMap<usize, LineIgnoreDirective>,
    scope: Scope,
    control_flow: ControlFlow,
    top_level_ctxt: SyntaxContext,
  ) -> Self {
    Self {
      file_name,
      source_map,
      program,
      file_ignore_directive,
      line_ignore_directives,
      scope,
      control_flow,
      top_level_ctxt,
      diagnostics: Vec::new(),
      plugin_codes: HashSet::new(),
    }
  }
  pub fn file_name(&self) -> &str {
    &self.file_name
  }

  pub fn diagnostics(&self) -> &[LintDiagnostic] {
    &self.diagnostics
  }

  pub fn plugin_codes(&self) -> &HashSet<String> {
    &self.plugin_codes
  }

  pub fn source_map(&self) -> Rc<SourceMap> {
    Rc::clone(&self.source_map)
  }

  pub fn program(&self) -> &AstView::Program<'view> {
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

  pub fn all_comments(&self) -> impl Iterator<Item = &'view Comment> {
    self
      .program
      .comments()
      .expect("Program should have information about comments, but doesn't")
      .all_comments()
  }

  pub fn leading_comments_at(
    &self,
    lo: BytePos,
  ) -> impl Iterator<Item = &'view Comment> {
    self
      .program
      .comments()
      .expect("Program should have information about comments, but doesn't")
      .leading_comments(lo)
  }

  pub fn trailing_comments_at(
    &self,
    hi: BytePos,
  ) -> impl Iterator<Item = &'view Comment> {
    self
      .program
      .comments()
      .expect("Program should have information about comments, but doesn't")
      .trailing_comments(hi)
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

      let diagnostic_line = diagnostic.range.start.line;
      if let Some(l) =
        self.line_ignore_directives.get_mut(&(diagnostic_line - 1))
      {
        if l.check_used(&diagnostic.code) {
          continue;
        }
      }

      filtered.push(diagnostic);
    }

    filtered
  }

  pub(crate) fn ban_unused_ignore(
    &self,
    specified_rules: &[Box<dyn LintRule>],
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
      let is_unknown = !executed_builtin_codes.contains(code.as_str())
        && !self.plugin_codes.contains(code.as_str());
      !status.used && !is_unknown
    };

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for (unused_code, _status) in
        file_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          file_ignore.span(),
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
          line_ignore.span(),
          CODE,
          format!("Ignore for code \"{}\" was not used.", unused_code),
          None,
        );
        diagnostics.push(d);
      }
    }

    diagnostics
  }

  pub(crate) fn ban_unknown_rule_code(&self) -> Vec<LintDiagnostic> {
    let builtin_all_rule_codes: HashSet<&'static str> =
      get_all_rules().into_iter().map(|r| r.code()).collect();
    let is_unknown_rule = |code: &&String| {
      !builtin_all_rule_codes.contains(code.as_str())
        && !self.plugin_codes.contains(code.as_str())
    };

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for unknown_rule_code in
        file_ignore.codes().keys().filter(is_unknown_rule)
      {
        let d = self.create_diagnostic(
          file_ignore.span(),
          "ban-unknown-rule-code",
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
          line_ignore.span(),
          "ban-unknown-rule-code",
          format!("Unknown rule for code \"{}\"", unknown_rule_code),
          None,
        );
        diagnostics.push(d);
      }
    }

    diagnostics
  }

  pub fn add_diagnostic(
    &mut self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
  ) {
    let diagnostic =
      self.create_diagnostic(span, code.to_string(), message.to_string(), None);
    self.diagnostics.push(diagnostic);
  }

  pub fn add_diagnostic_with_hint(
    &mut self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
    hint: impl ToString,
  ) {
    let diagnostic =
      self.create_diagnostic(span, code, message, Some(hint.to_string()));
    self.diagnostics.push(diagnostic);
  }

  pub(crate) fn create_diagnostic(
    &self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
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

  pub fn set_plugin_codes(&mut self, codes: HashSet<String>) {
    self.plugin_codes = codes;
  }
}
