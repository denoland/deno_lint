// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::Expr::{Assign, Bin, Paren};
use crate::swc_ecma_ast::Module;
use swc_ecmascript::visit::{Node, Visit};

use std::sync::Arc;

pub struct NoCondAssign;

impl LintRule for NoCondAssign {
  fn new() -> Box<Self> {
    Box::new(NoCondAssign)
  }

  fn code(&self) -> &'static str {
    "no-cond-assign"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoCondAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoCondAssignVisitor {
  context: Arc<Context>,
}

impl NoCondAssignVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-cond-assign",
      "Expected a conditional expression and instead saw an assignment",
    );
  }

  fn check_condition(&self, condition: &Expr) {
    match condition {
      Assign(assign) => {
        self.add_diagnostic(assign.span);
      }
      Bin(bin) => {
        if bin.op == swc_ecma_ast::BinaryOp::LogicalOr {
          self.check_condition(&bin.left);
          self.check_condition(&bin.right);
        }
      }
      _ => {}
    }
  }
}

impl Visit for NoCondAssignVisitor {
  fn visit_if_stmt(
    &mut self,
    if_stmt: &swc_ecma_ast::IfStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&if_stmt.test);
  }
  fn visit_while_stmt(
    &mut self,
    while_stmt: &swc_ecma_ast::WhileStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&while_stmt.test);
  }
  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &swc_ecma_ast::DoWhileStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&do_while_stmt.test);
  }
  fn visit_for_stmt(
    &mut self,
    for_stmt: &swc_ecma_ast::ForStmt,
    _parent: &dyn Node,
  ) {
    if let Some(for_test) = &for_stmt.test {
      self.check_condition(&for_test);
    }
  }
  fn visit_cond_expr(
    &mut self,
    cond_expr: &swc_ecma_ast::CondExpr,
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
  use crate::test_util::*;

  #[test]
  fn it_passes_using_equality_operator() {
    assert_lint_ok::<NoCondAssign>("if (x === 0) { };");
  }

  #[test]
  fn it_passes_with_bracketed_assignment() {
    assert_lint_ok::<NoCondAssign>("if ((x = y)) { }");
  }

  #[test]
  fn it_fails_using_assignment_in_if_stmt() {
    assert_lint_err::<NoCondAssign>("if (x = 0) { }", 4);
  }

  #[test]
  fn it_fails_using_assignment_in_while_stmt() {
    assert_lint_err::<NoCondAssign>("while (x = 0) { }", 7);
  }

  #[test]
  fn it_fails_using_assignment_in_do_while_stmt() {
    assert_lint_err::<NoCondAssign>("do { } while (x = 0);", 14);
  }

  #[test]
  fn it_fails_using_assignment_in_for_stmt() {
    assert_lint_err::<NoCondAssign>("for (let i = 0; i = 10; i++) { }", 16);
  }

  #[test]
  fn no_cond_assign_valid() {
    assert_lint_ok::<NoCondAssign>("const x = 0; if (x == 0) { const b = 1; }");
    assert_lint_ok::<NoCondAssign>("const x = 5; while (x < 5) { x = x + 1; }");
    assert_lint_ok::<NoCondAssign>("while ((a = b));");
    assert_lint_ok::<NoCondAssign>("do {} while ((a = b));");
    assert_lint_ok::<NoCondAssign>("for (;(a = b););");
    assert_lint_ok::<NoCondAssign>("for (;;) {}");
    assert_lint_ok::<NoCondAssign>(
      "if (someNode || (someNode = parentNode)) { }",
    );
    assert_lint_ok::<NoCondAssign>(
      "while (someNode || (someNode = parentNode)) { }",
    );
    assert_lint_ok::<NoCondAssign>(
      "do { } while (someNode || (someNode = parentNode));",
    );
    assert_lint_ok::<NoCondAssign>(
      "for (;someNode || (someNode = parentNode););",
    );
    assert_lint_ok::<NoCondAssign>(
      "if ((function(node) { return node = parentNode; })(someNode)) { }",
    );
    assert_lint_ok::<NoCondAssign>(
      "if ((node => node = parentNode)(someNode)) { }",
    );
    assert_lint_ok::<NoCondAssign>(
      "if (function(node) { return node = parentNode; }) { }",
    );
    assert_lint_ok::<NoCondAssign>("const x; const b = (x === 0) ? 1 : 0;");
    assert_lint_ok::<NoCondAssign>("switch (foo) { case a = b: bar(); }");
  }

  #[test]
  fn no_cond_assign_invalid() {
    assert_lint_err::<NoCondAssign>("const x; if (x = 0) { const b = 1; }", 13);
    assert_lint_err::<NoCondAssign>(
      "const x; while (x = 0) { const b = 1; }",
      16,
    );
    assert_lint_err::<NoCondAssign>(
      "const x = 0, y; do { y = x; } while (x = x + 1);",
      37,
    );
    assert_lint_err::<NoCondAssign>("let x; for(; x+=1 ;){};", 13);
    assert_lint_err::<NoCondAssign>("let x; if ((x) = (0));", 11);
    assert_lint_err::<NoCondAssign>("let x; let b = (x = 0) ? 1 : 0;", 16);
    assert_lint_err::<NoCondAssign>(
      "(((123.45)).abcd = 54321) ? foo : bar;",
      1,
    );
  }
}
