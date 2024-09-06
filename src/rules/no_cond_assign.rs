// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CondExpr, DoWhileStmt, Expr, ForStmt, IfStmt, WhileStmt};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoCondAssign;

const CODE: &str = "no-cond-assign";

#[derive(Display)]
enum NoCondAssignMessage {
  #[display("Expected a conditional expression and instead saw an assignment")]
  Unexpected,
}

#[derive(Display)]
enum NoCondAssignHint {
  #[display(
    "Change assignment (`=`) to comparison (`===`) or move assignment out of condition"
  )]
  ChangeOrMove,
}

impl LintRule for NoCondAssign {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoCondAssignHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_cond_assign.md")
  }
}

struct NoCondAssignHandler;

impl NoCondAssignHandler {
  fn add_diagnostic(&mut self, range: SourceRange, ctx: &mut Context) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      NoCondAssignMessage::Unexpected,
      NoCondAssignHint::ChangeOrMove,
    );
  }

  fn check_condition(&mut self, condition: &Expr, ctx: &mut Context) {
    match condition {
      Expr::Assign(assign) => {
        self.add_diagnostic(assign.range(), ctx);
      }
      Expr::Bin(bin) => {
        if bin.op() == deno_ast::swc::ast::BinaryOp::LogicalOr {
          self.check_condition(&bin.left, ctx);
          self.check_condition(&bin.right, ctx);
        }
      }
      _ => {}
    }
  }
}

impl Handler for NoCondAssignHandler {
  fn if_stmt(&mut self, if_stmt: &IfStmt, ctx: &mut Context) {
    self.check_condition(&if_stmt.test, ctx);
  }

  fn while_stmt(&mut self, while_stmt: &WhileStmt, ctx: &mut Context) {
    self.check_condition(&while_stmt.test, ctx);
  }

  fn do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt, ctx: &mut Context) {
    self.check_condition(&do_while_stmt.test, ctx);
  }

  fn for_stmt(&mut self, for_stmt: &ForStmt, ctx: &mut Context) {
    if let Some(for_test) = &for_stmt.test {
      self.check_condition(for_test, ctx);
    }
  }

  fn cond_expr(&mut self, cond_expr: &CondExpr, ctx: &mut Context) {
    if let Expr::Paren(paren) = cond_expr.test {
      self.check_condition(&paren.expr, ctx);
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
