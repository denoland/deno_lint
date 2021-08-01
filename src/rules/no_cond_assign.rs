// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_common::Span;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Expr::{Assign, Bin, Paren};
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

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
  fn new() -> Box<Self> {
    Box::new(NoCondAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoCondAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_cond_assign.md")
  }
}

struct NoCondAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoCondAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoCondAssignMessage::Unexpected,
      NoCondAssignHint::ChangeOrMove,
    );
  }

  fn check_condition(&mut self, condition: &Expr) {
    match condition {
      Assign(assign) => {
        self.add_diagnostic(assign.span);
      }
      Bin(bin) => {
        if bin.op == swc_ecmascript::ast::BinaryOp::LogicalOr {
          self.check_condition(&bin.left);
          self.check_condition(&bin.right);
        }
      }
      _ => {}
    }
  }
}

impl<'c, 'view> VisitAll for NoCondAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_if_stmt(
    &mut self,
    if_stmt: &swc_ecmascript::ast::IfStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&if_stmt.test);
  }

  fn visit_while_stmt(
    &mut self,
    while_stmt: &swc_ecmascript::ast::WhileStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&while_stmt.test);
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &swc_ecmascript::ast::DoWhileStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&do_while_stmt.test);
  }

  fn visit_for_stmt(
    &mut self,
    for_stmt: &swc_ecmascript::ast::ForStmt,
    _parent: &dyn Node,
  ) {
    if let Some(for_test) = &for_stmt.test {
      self.check_condition(for_test);
    }
  }

  fn visit_cond_expr(
    &mut self,
    cond_expr: &swc_ecmascript::ast::CondExpr,
    _parent: &dyn Node,
  ) {
    if let Paren(paren) = &*cond_expr.test {
      self.check_condition(&paren.expr);
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
