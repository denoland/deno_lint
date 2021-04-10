use crate::control_flow::ControlFlow;
use crate::diagnostic::{LintDiagnostic, Position, Range};
use crate::ignore_directives::IgnoreDirective;
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view as AstView;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Instant;
use swc_common::{SourceMap, Span, SyntaxContext};

pub struct Context<'view> {
  pub file_name: String,
  pub diagnostics: Vec<LintDiagnostic>,
  pub(crate) plugin_codes: HashSet<String>,
  pub source_map: Rc<SourceMap>,
  pub(crate) program: AstView::Program<'view>,
  pub ignore_directives: RefCell<Vec<IgnoreDirective>>,
  pub(crate) scope: Scope,
  // TODO(magurotuna): Making control_flow public is just needed for implementing plugin prototype.
  // It will be likely possible to revert it to `pub(crate)` later.
  pub control_flow: ControlFlow,
  pub(crate) top_level_ctxt: SyntaxContext,
}

impl<'view> Context<'view> {
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
