// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::BinExpr;
use swc_ecma_ast::BinaryOp;
use swc_ecma_ast::Expr;
use swc_ecma_ast::UnaryOp;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoUnsafeNegation;

impl LintRule for NoUnsafeNegation {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeNegation)
  }

  fn code(&self) -> &'static str {
    "noUnsafeNegation"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoUnsafeNegationVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoUnsafeNegationVisitor {
  context: Context,
}

impl NoUnsafeNegationVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoUnsafeNegationVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if bin_expr.op == BinaryOp::In || bin_expr.op == BinaryOp::InstanceOf {
      if let Expr::Unary(unary_expr) = &*bin_expr.left {
        if unary_expr.op == UnaryOp::Bang {
          self.context.add_diagnostic(
            bin_expr.span,
            "noUnsafeNegation",
            "Unexpected negation of left operand",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unsafe_negation_ok() {
    assert_lint_ok_n::<NoUnsafeNegation>(vec![
      "1 in [1, 2, 3]",
      "key in object",
      "foo instanceof Date",
      "!(1 in [1, 2, 3])",
      "!(key in object)",
      "!(foo instanceof Date)",
    ]);
  }

  #[test]
  fn no_unsafe_negation() {
    assert_lint_err::<NoUnsafeNegation>("!1 in [1, 2, 3]", 0);
    assert_lint_err::<NoUnsafeNegation>("!key in object", 0);
    assert_lint_err::<NoUnsafeNegation>("!foo instanceof Date", 0);
  }
}
