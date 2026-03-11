// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoCondAssign;

const CODE: &str = "no-cond-assign";

#[derive(Display)]
enum NoCondAssignMessage {
  #[display(
    fmt = "Expected a conditional expression and instead saw an assignment"
  )]
  Unexpected,
}

#[derive(Display)]
enum NoCondAssignHint {
  #[display(
    fmt = "Change assignment (`=`) to comparison (`===`) or move assignment out of condition"
  )]
  ChangeOrMove,
}

impl LintRule for NoCondAssign {
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
    let mut handler = NoCondAssignHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoCondAssignHandler;

impl NoCondAssignHandler {
  fn check_condition(&mut self, condition: &Expression, ctx: &mut Context) {
    match condition {
      Expression::AssignmentExpression(assign) => {
        ctx.add_diagnostic_with_hint(
          assign.span,
          CODE,
          NoCondAssignMessage::Unexpected,
          NoCondAssignHint::ChangeOrMove,
        );
      }
      Expression::LogicalExpression(log) => {
        if log.operator == LogicalOperator::Or {
          self.check_condition(&log.left, ctx);
          self.check_condition(&log.right, ctx);
        }
      }
      _ => {}
    }
  }
}

impl Handler<'_> for NoCondAssignHandler {
  fn if_statement(&mut self, if_stmt: &IfStatement, ctx: &mut Context) {
    self.check_condition(&if_stmt.test, ctx);
  }

  fn while_statement(
    &mut self,
    while_stmt: &WhileStatement,
    ctx: &mut Context,
  ) {
    self.check_condition(&while_stmt.test, ctx);
  }

  fn do_while_statement(
    &mut self,
    do_while_stmt: &DoWhileStatement,
    ctx: &mut Context,
  ) {
    self.check_condition(&do_while_stmt.test, ctx);
  }

  fn for_statement(&mut self, for_stmt: &ForStatement, ctx: &mut Context) {
    if let Some(for_test) = &for_stmt.test {
      self.check_condition(for_test, ctx);
    }
  }

  fn conditional_expression(
    &mut self,
    cond_expr: &ConditionalExpression,
    ctx: &mut Context,
  ) {
    if let Expression::ParenthesizedExpression(paren) = &cond_expr.test {
      self.check_condition(&paren.expression, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_cond_assign_valid() {
    assert_lint_ok! {
      NoCondAssign,
      "if (x === 0) { };",
      "if ((x = y)) { }",
      "const x = 0; if (x == 0) { const b = 1; }",
      "const x = 5; while (x < 5) { x = x + 1; }",
      "while ((a = b));",
      "do {} while ((a = b));",
      "for (;(a = b););",
      "for (;;) {}",
      "if (someNode || (someNode = parentNode)) { }",
      "while (someNode || (someNode = parentNode)) { }",
      "do { } while (someNode || (someNode = parentNode));",
      "for (;someNode || (someNode = parentNode););",
      "if ((function(node) { return node = parentNode; })(someNode)) { }",
      "if ((node => node = parentNode)(someNode)) { }",
      "if (function(node) { return node = parentNode; }) { }",
      "const x; const b = (x === 0) ? 1 : 0;",
      "switch (foo) { case a = b: bar(); }",
    };
  }

  #[test]
  fn no_cond_assign_invalid() {
    assert_lint_err! {
      NoCondAssign,
      "if (x = 0) { }": [
        {
          col: 4,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "while (x = 0) { }": [
        {
          col: 7,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "do { } while (x = 0);": [
        {
          col: 14,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "for (let i = 0; i = 10; i++) { }": [
        {
          col: 16,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "const x; if (x = 0) { const b = 1; }": [
        {
          col: 13,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "const x; while (x = 0) { const b = 1; }": [
        {
          col: 16,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "const x = 0, y; do { y = x; } while (x = x + 1);": [
        {
          col: 37,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "let x; for(; x+=1 ;){};": [
        {
          col: 13,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "let x; if ((x) = (0));": [
        {
          col: 11,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "let x; let b = (x = 0) ? 1 : 0;": [
        {
          col: 16,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "(((123.45)).abcd = 54321) ? foo : bar;": [
        {
          col: 1,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],

      // nested
      "if (foo) { if (x = 0) {} }": [
        {
          col: 15,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "while (foo) { while (x = 0) {} }": [
        {
          col: 21,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "do { do {} while (x = 0) } while (foo);": [
        {
          col: 18,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "for (let i = 0; i < 10; i++) { for (; j+=1 ;) {} }": [
        {
          col: 38,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ],
      "const val = foo ? (x = 0) ? 0 : 1 : 2;": [
        {
          col: 19,
          message: NoCondAssignMessage::Unexpected,
          hint: NoCondAssignHint::ChangeOrMove,
        }
      ]
    };
  }
}
