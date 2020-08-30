// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::BinExpr;
use swc_ecmascript::ast::BinaryOp;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoUnsafeNegation;

impl LintRule for NoUnsafeNegation {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeNegation)
  }

  fn code(&self) -> &'static str {
    "no-unsafe-negation"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoUnsafeNegationVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoUnsafeNegationVisitor {
  context: Arc<Context>,
}

impl NoUnsafeNegationVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoUnsafeNegationVisitor {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if bin_expr.op == BinaryOp::In || bin_expr.op == BinaryOp::InstanceOf {
      if let Expr::Unary(unary_expr) = &*bin_expr.left {
        if unary_expr.op == UnaryOp::Bang {
          self.context.add_diagnostic(
            bin_expr.span,
            "no-unsafe-negation",
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
