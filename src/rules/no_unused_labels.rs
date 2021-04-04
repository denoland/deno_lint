// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use derive_more::Display;
use dprint_swc_ecma_ast_view as AstView;
use if_chain::if_chain;
use swc_common::Spanned;

pub struct NoUnusedLabels;

const CODE: &str = "no-unused-labels";

#[derive(Display)]
enum NoUnusedLabelsMessage {
  #[display(fmt = "`{}` label is never used", _0)]
  Unused(String),
}

impl LintRule for NoUnusedLabels {
  fn new() -> Box<Self> {
    Box::new(NoUnusedLabels)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: AstView::Program,
  ) {
    let mut handler = NoUnusedLabelsHandler::default();
    handler.traverse(program, context);
  }
}

struct Label {
  used: bool,
  name: String,
}

#[derive(Default)]
struct NoUnusedLabelsHandler {
  labels: Vec<Label>,
}

impl NoUnusedLabelsHandler {
  fn check_label(&mut self, label: Option<&AstView::Ident>) {
    if let Some(label) = label {
      if let Some(found) = self
        .labels
        .iter_mut()
        .rfind(|l| l.name.as_str() == label.sym())
      {
        found.used = true;
      }
    }
  }
}

impl Handler for NoUnusedLabelsHandler {
  fn labeled_stmt(
    &mut self,
    labeled_stmt: &AstView::LabeledStmt,
    _ctx: &mut Context,
  ) {
    self.labels.push(Label {
      used: false,
      name: labeled_stmt.label.sym().to_string(),
    });
  }

  fn continue_stmt(
    &mut self,
    continue_stmt: &AstView::ContinueStmt,
    _ctx: &mut Context,
  ) {
    self.check_label(continue_stmt.label);
  }

  fn break_stmt(
    &mut self,
    break_stmt: &AstView::BreakStmt,
    _ctx: &mut Context,
  ) {
    self.check_label(break_stmt.label);
  }

  fn on_exit_node(&mut self, node: AstView::Node, ctx: &mut Context) {
    if_chain! {
      if let Some(ref labeled_stmt) = node.to::<AstView::LabeledStmt>();
      if let Some(label) = self.labels.pop();
      if !label.used;
      then {
        ctx.add_diagnostic(
          labeled_stmt.span(),
          CODE,
          NoUnusedLabelsMessage::Unused(label.name),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unused_label_valid() {
    assert_lint_ok! {
      NoUnusedLabels,
      "LABEL: for (let i = 0; i < 5; i++) { a(); break LABEL; }",
      "LABEL: for (let i = 0; i < 5; i++) { a(); if (i < 3) { continue LABEL; } b(); if (i > 3) { break LABEL; } }",
      "LABEL: { a(); b(); break LABEL; c(); }",
      "A: { B: break B; C: for (var i = 0; i < 10; ++i) { foo(); if (a) break A; if (c) continue C; bar(); } }",
      "LABEL: while(true) { break LABEL; }",
      "LABEL: break LABEL;",
    };
  }

  #[test]
  fn no_unused_label_invalid() {
    assert_lint_err! {
      NoUnusedLabels,
      "LABEL: var a = 0;": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "LABEL: if (something) { a(); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "LABEL: for (let i = 0; i < 5; i++) { a(); b(); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "A: for (var i = 0; i < 10; ++i) { B: break A; }": [
        {
          col: 34,
          message: variant!(NoUnusedLabelsMessage, Unused, "B"),
        }
      ],
      "A: { let A = 0; console.log(A); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "A"),
        }
      ],
    };
  }
}
