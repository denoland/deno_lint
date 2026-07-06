// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  BreakStatement, ContinueStatement, LabeledStatement, Program,
};
use derive_more::Display;

#[derive(Debug)]
pub struct NoUnusedLabels;

const CODE: &str = "no-unused-labels";

#[derive(Display)]
enum NoUnusedLabelsMessage {
  #[display(fmt = "`{}` label is never used", _0)]
  Unused(String),
}

impl LintRule for NoUnusedLabels {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoUnusedLabelsHandler::default();
    crate::handler::traverse_program(&mut handler, program, context);
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
  fn check_label(
    &mut self,
    label: Option<&deno_ast::oxc::ast::ast::LabelIdentifier>,
  ) {
    if let Some(label) = label {
      if let Some(found) = self
        .labels
        .iter_mut()
        .rfind(|l| l.name.as_str() == label.name.as_str())
      {
        found.used = true;
      }
    }
  }
}

impl Handler<'_> for NoUnusedLabelsHandler {
  fn labeled_statement(
    &mut self,
    labeled_stmt: &LabeledStatement,
    _ctx: &mut Context,
  ) {
    self.labels.push(Label {
      used: false,
      name: labeled_stmt.label.name.to_string(),
    });
  }

  fn labeled_statement_exit(
    &mut self,
    labeled_stmt: &LabeledStatement,
    ctx: &mut Context,
  ) {
    if let Some(label) = self.labels.pop() {
      if !label.used {
        ctx.add_diagnostic(
          labeled_stmt.span,
          CODE,
          NoUnusedLabelsMessage::Unused(label.name),
        );
      }
    }
  }

  fn continue_statement(
    &mut self,
    continue_stmt: &ContinueStatement,
    _ctx: &mut Context,
  ) {
    self.check_label(continue_stmt.label.as_ref());
  }

  fn break_statement(
    &mut self,
    break_stmt: &BreakStatement,
    _ctx: &mut Context,
  ) {
    self.check_label(break_stmt.label.as_ref());
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
