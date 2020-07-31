// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{BinExpr, BinaryOp};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct Eqeqeq;

impl LintRule for Eqeqeq {
  fn new() -> Box<Self> {
    Box::new(Eqeqeq)
  }

  fn code(&self) -> &'static str {
    "eqeqeq"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = EqeqeqVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct EqeqeqVisitor {
  context: Arc<Context>,
}

impl EqeqeqVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for EqeqeqVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, parent: &dyn Node) {
    if matches!(bin_expr.op, BinaryOp::EqEq | BinaryOp::NotEq) {
      let message = if bin_expr.op == BinaryOp::EqEq {
        "expected '===' and instead saw '=='."
      } else {
        "expected '!==' and instead saw '!='."
      };
      self
        .context
        .add_diagnostic(bin_expr.span, "eqeqeq", message)
    }
    swc_ecmascript::visit::visit_bin_expr(self, bin_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn eqeqeq_valid() {
    assert_lint_ok::<Eqeqeq>("midori === sapphire");
    assert_lint_ok::<Eqeqeq>("midori !== hazuki");
    assert_lint_ok::<Eqeqeq>("kumiko === null");
    assert_lint_ok::<Eqeqeq>("reina !== null");
    assert_lint_ok::<Eqeqeq>("null === null");
    assert_lint_ok::<Eqeqeq>("null !== null");
  }

  #[test]
  fn eqeqeq_invalid() {
    assert_lint_err::<Eqeqeq>("a == b", 0);
    assert_lint_err::<Eqeqeq>("a != b", 0);
    assert_lint_err::<Eqeqeq>("typeof a == 'number'", 0);
    assert_lint_err::<Eqeqeq>("'string' != typeof a", 0);
    assert_lint_err::<Eqeqeq>("true == true", 0);
    assert_lint_err::<Eqeqeq>("2 == 3", 0);
    assert_lint_err::<Eqeqeq>("'hello' != 'world'", 0);
    assert_lint_err::<Eqeqeq>("a == null", 0);
    assert_lint_err::<Eqeqeq>("null != a", 0);
    assert_lint_err::<Eqeqeq>("true == null", 0);
    assert_lint_err::<Eqeqeq>("true != null", 0);
    assert_lint_err::<Eqeqeq>("null == null", 0);
    assert_lint_err::<Eqeqeq>("null != null", 0);
    assert_lint_err_on_line::<Eqeqeq>(
      r#"
a
==
b"#,
      2,
      0,
    );
    assert_lint_err::<Eqeqeq>("(a) == b", 0);
    assert_lint_err::<Eqeqeq>("(a) != b", 0);
    assert_lint_err::<Eqeqeq>("a == (b)", 0);
    assert_lint_err::<Eqeqeq>("a != (b)", 0);
    assert_lint_err::<Eqeqeq>("(a) == (b)", 0);
    assert_lint_err::<Eqeqeq>("(a) != (b)", 0);
    assert_lint_err_n::<Eqeqeq>("(a == b) == (c)", vec![0, 1]);
    assert_lint_err_n::<Eqeqeq>("(a != b) != (c)", vec![0, 1]);
    assert_lint_err::<Eqeqeq>("(a == b) === (c)", 1);
    assert_lint_err::<Eqeqeq>("(a == b) !== (c)", 1);
    assert_lint_err::<Eqeqeq>("(a === b) == (c)", 0);
    assert_lint_err::<Eqeqeq>("(a === b) != (c)", 0);
    assert_lint_err::<Eqeqeq>("a == b;", 0);
    assert_lint_err::<Eqeqeq>("a!=b;", 0);
    assert_lint_err::<Eqeqeq>("(a + b) == c;", 0);
    assert_lint_err::<Eqeqeq>("(a + b)  !=  c;", 0);
    assert_lint_err::<Eqeqeq>("((1) )  ==  (2);", 0);
  }
}
