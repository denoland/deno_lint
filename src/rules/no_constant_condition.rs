// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::Module;
use swc_ecma_visit::{Node, Visit};

pub struct NoConstantCondition;

impl LintRule for NoConstantCondition {
  fn new() -> Box<Self> {
    Box::new(NoConstantCondition)
  }

  fn code(&self) -> &'static str {
    "no-constant-condition"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoConstantConditionVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoConstantConditionVisitor {
  context: Context,
}

impl NoConstantConditionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-constant-condition",
      "Use of a constant expressions as conditions is not allowed.",
    );
  }

  fn check_short_circuit(
    &self,
    expr: &Expr,
    operator: swc_ecma_ast::BinaryOp,
  ) -> bool {
    match expr {
      Expr::Lit(boolean) => {
        (operator == swc_ecma_ast::BinaryOp::LogicalOr
          && &boolean.value == true)
          || (operator == swc_ecma_ast::BinaryOp::LogicalAnd
            && boolean.value == false)
      }
      Expr::Unary(unary) => {
        operator == swc_ecma_ast::BinaryOp::LogicalAnd
          && unary.op == swc_ecma_ast::UnaryOp::Void
      }
      Expr::Bin(bin)
        if bin.op == swc_ecma_ast::BinaryOp::LogicalAnd
          || bin.op == swc_ecma_ast::BinaryOp::LogicalOr =>
      {
        self.check_short_circuit(&bin.left, bin.op)
          || self.check_short_circuit(&bin.right, bin.op)
      }
      _ => false,
    }
  }

  fn is_constant(&self, condition: &Expr) -> (bool, Option<Span>) {
    match condition {
      Expr::Lit(lit) => (true, Some(lit.span())),
      //     // TODO(humancalico)
      //     Expr::Bin(bin) => {
      //       if bin.op == swc_ecma_ast::BinaryOp::LogicalOr || bin.op == swc_ecma_ast::BinaryOp::LogicalAnd {
      //         let is_left_constant = self.is_constant(&bin.left).0;
      //         let is_right_constant = self.is_constant(&bin.right).0;
      //         let is_left_short_circuit = is_left_constant && self.check_short_circuit(&bin.left, bin.op);
      //         let is_right_short_circuit = is_right_constant && self.check_short_circuit(&bin.right, bin.op);
      //       }
      //     }
      //       // ----
      Expr::Assign(assign) => self.is_constant(&assign.right),
      Expr::Seq(seq) => self.is_constant(&seq.exprs[seq.exprs.len() - 1]),
      _ => (false, None),
    }
  }
}

impl Visit for NoConstantConditionVisitor {
  fn visit_if_stmt(
    &mut self,
    if_stmt: &swc_ecma_ast::IfStmt,
    _parent: &dyn Node,
  ) {
    let const_result = self.is_constant(&if_stmt.test);
    if const_result.0 {
      self.add_diagnostic(const_result.1.unwrap())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_constant_condition_1() {
    assert_lint_err::<NoConstantCondition>(r#"if ("some str") {}"#, 4);
  }
}
