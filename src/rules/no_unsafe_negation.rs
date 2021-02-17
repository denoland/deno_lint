// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::BinExpr;
use swc_ecmascript::ast::BinaryOp;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoUnsafeNegation;

impl LintRule for NoUnsafeNegation {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeNegation)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-unsafe-negation"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: ProgramRef<'_>,
  ) {
    let mut visitor = NoUnsafeNegationVisitor::new(context);
    match program {
        ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
        ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct NoUnsafeNegationVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoUnsafeNegationVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoUnsafeNegationVisitor<'c> {
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
  fn no_unsafe_negation_valid() {
    assert_lint_ok! {
      NoUnsafeNegation,
      "1 in [1, 2, 3]",
      "key in object",
      "foo instanceof Date",
      "!(1 in [1, 2, 3])",
      "!(key in object)",
      "!(foo instanceof Date)",
    };
  }

  #[test]
  fn no_unsafe_negation_invalid() {
    assert_lint_err::<NoUnsafeNegation>("!1 in [1, 2, 3]", 0);
    assert_lint_err::<NoUnsafeNegation>("!key in object", 0);
    assert_lint_err::<NoUnsafeNegation>("!foo instanceof Date", 0);
  }
}
