// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::Lit;
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

  fn _check_short_circuit(
    &self,
    expr: &Expr,
    operator: swc_ecma_ast::BinaryOp,
  ) -> bool {
    match expr {
      Expr::Lit(lit) => match lit {
        Lit::Bool(boolean) => {
          (operator == swc_ecma_ast::BinaryOp::LogicalOr && boolean.value)
            || (operator == swc_ecma_ast::BinaryOp::LogicalAnd && boolean.value)
        }
        _ => false,
      },
      Expr::Unary(unary) => {
        operator == swc_ecma_ast::BinaryOp::LogicalAnd
          && unary.op == swc_ecma_ast::UnaryOp::Void
      }
      Expr::Bin(bin)
        if bin.op == swc_ecma_ast::BinaryOp::LogicalAnd
          || bin.op == swc_ecma_ast::BinaryOp::LogicalOr =>
      {
        self._check_short_circuit(&bin.left, bin.op)
          || self._check_short_circuit(&bin.right, bin.op)
      }
      _ => false,
    }
  }

  fn is_constant(
    &self,
    node: &Expr,
    _parent_node: Option<&Expr>,
  ) -> (bool, Option<Span>) {
    match node {
      Expr::Lit(lit) => (true, Some(lit.span())),
      Expr::Unary(unary) => {
        if unary.op == swc_ecma_ast::UnaryOp::Void {
          (true, Some(unary.span))
        } else {
          self.is_constant(&unary.arg, Some(unary))
        }
      }
      // TODO(humancalico)
      Expr::Bin(bin) => {
        /*
        if bin.op == swc_ecma_ast::BinaryOp::LogicalOr
          || bin.op == swc_ecma_ast::BinaryOp::LogicalAnd
        {
          let is_left_constant = self.is_constant(&bin.left, Some(node)).0;
          let is_right_constant = self.is_constant(&bin.right, Some(node)).0;
          let _is_left_short_circuit =
            is_left_constant && self._check_short_circuit(&bin.left, bin.op);
          let _is_right_short_circuit =
            is_right_constant && self._check_short_circuit(&bin.right, bin.op);
          let to_return: bool = (is_left_constant && is_right_constant)
            || (if let swc_ecma_ast::Lit(lit) = bin.right {
              bin.op != swc_ecma_ast::BinaryOp::LogicalOr && is_right_constant
            });
          // (is_left_constant && is_right_constant) || ();
          (false, None)
        } else */
        if bin.op == swc_ecma_ast::BinaryOp::In {
          (
            self.is_constant(&bin.left, Some(node)).0
              && self.is_constant(&bin.right, Some(node)).0,
            Some(bin.span),
          )
        } else {
          (false, None)
        }
      }
      // ----
      Expr::Assign(assign) => self.is_constant(&assign.right, Some(node)),
      Expr::Seq(seq) => {
        self.is_constant(&seq.exprs[seq.exprs.len() - 1], Some(node))
      }
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
    let const_result = self.is_constant(&if_stmt.test, None);
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
    assert_lint_err::<NoConstantCondition>(r#"if (-2) {}"#, 4);
  }
}
