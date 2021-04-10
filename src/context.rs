use crate::control_flow::ControlFlow;
use crate::diagnostic::{LintDiagnostic, Position, Range};
use crate::ignore_directives::IgnoreDirective;
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view::{
  self as AstView, BytePos, RootNode, SpannedExt,
};
use std::collections::HashSet;
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
  ignore_directives: Vec<IgnoreDirective>,
  scope: Scope,
  control_flow: ControlFlow,
  top_level_ctxt: SyntaxContext,
}

impl<'view> Context<'view> {
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

  pub fn ignore_directives(&self) -> &[IgnoreDirective] {
    &self.ignore_directives
  }

  pub fn ignore_directives_mut(&mut self) -> &mut [IgnoreDirective] {
    &mut self.ignore_directives
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

  pub fn all_leading_comments(&self) -> impl Iterator<Item = &'view Comment> {
    self.program.leading_comments_fast(&self.program)
  }

  pub fn all_trailing_comments(&self) -> impl Iterator<Item = &'view Comment> {
    self.program.trailing_comments_fast(&self.program)
  }

  pub fn all_comments(&self) -> impl Iterator<Item = &'view Comment> {
    self
      .all_leading_comments()
      .chain(self.all_trailing_comments())
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
