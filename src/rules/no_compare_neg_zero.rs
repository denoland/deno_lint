// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_ecma_ast::BinaryOp::*;
use crate::swc_ecma_ast::Expr::{Lit, Unary};
use crate::swc_ecma_ast::Lit::Num;
use crate::swc_ecma_ast::UnaryExpr;
use crate::swc_ecma_ast::UnaryOp::Minus;
use crate::swc_ecma_ast::{BinExpr, BinaryOp, Expr, Module};
use swc_ecma_visit::{Node, Visit};

pub struct NoCompareNegZero;

impl LintRule for NoCompareNegZero {
  fn new() -> Box<Self> {
    Box::new(NoCompareNegZero)
  }

  fn code(&self) -> &'static str {
    "no-compare-neg-zero"
  }

  fn lint_module(&self, context: Context, module: &Module) {
    let mut visitor = NoCompareNegZeroVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoCompareNegZeroVisitor {
  context: Context,
}

impl NoCompareNegZeroVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoCompareNegZeroVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if !bin_expr.op.is_comparator() {
      return;
    }

    if bin_expr.left.is_neg_zero() || bin_expr.right.is_neg_zero() {
      self.context.add_diagnostic(
        bin_expr.span,
        "no-compare-neg-zero",
        "Do not compare against -0",
      );
    }
  }
}

trait Comparator {
  fn is_comparator(&self) -> bool;
}

impl Comparator for BinaryOp {
  fn is_comparator(&self) -> bool {
    match self {
      EqEq | NotEq | EqEqEq | NotEqEq | Lt | LtEq | Gt | GtEq => true,
      _ => false,
    }
  }
}

trait NegZero {
  fn is_neg_zero(&self) -> bool;
}

impl NegZero for Expr {
  fn is_neg_zero(&self) -> bool {
    match self {
      Unary(unary) => unary.is_neg_zero(),
      _ => false,
    }
  }
}

impl NegZero for UnaryExpr {
  fn is_neg_zero(&self) -> bool {
    if let (Minus, Lit(Num(number))) = (self.op, &*self.arg) {
      return number.value == 0.0;
    }
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_using_positive_zero() {
    assert_lint_ok::<NoCompareNegZero>("if (x === 0) { }");
  }

  #[test]
  fn it_passes_using_object_is_neg_zero() {
    assert_lint_ok::<NoCompareNegZero>("if (Object.is(x, -0)) { }");
  }

  #[test]
  fn it_fails_using_double_eq_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x == -0) { }", 4);
  }

  #[test]
  fn it_fails_using_not_eq_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x != -0) { }", 4);
  }

  #[test]
  fn it_fails_using_triple_eq_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x === -0) { }", 4);
  }

  #[test]
  fn it_fails_using_not_double_eq_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x !== -0) { }", 4);
  }

  #[test]
  fn it_fails_using_less_than_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x < -0) { }", 4);
  }

  #[test]
  fn it_fails_using_less_than_eq_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x <= -0) { }", 4);
  }

  #[test]
  fn it_fails_using_greater_than_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x > -0) { }", 4);
  }

  #[test]
  fn it_fails_using_greater_than_equal_with_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x >= -0) { }", 4);
  }

  #[test]
  fn it_fails_with_neg_zero_as_the_left_operand() {
    assert_lint_err::<NoCompareNegZero>("if (-0 == x) { }", 4);
  }

  #[test]
  fn it_fails_with_floating_point_neg_zero() {
    assert_lint_err::<NoCompareNegZero>("if (x == -0.0) { }", 4);
  }
}
