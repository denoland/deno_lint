// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{BinExpr, BinaryOp, Expr, Lit};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct Eqeqeq;

impl LintRule for Eqeqeq {
  fn new() -> Box<Self> {
    Box::new(Eqeqeq)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = EqeqeqVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct EqeqeqVisitor {
  context: Context,
}

impl EqeqeqVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

fn is_null(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(lit) => matches!(lit, Lit::Null(_)),
    _ => false,
  }
}

impl Visit for EqeqeqVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if bin_expr.op == BinaryOp::EqEq || bin_expr.op == BinaryOp::NotEq {
      if is_null(&bin_expr.left) || is_null(&bin_expr.right) {
        return;
      }

      let message = if bin_expr.op == BinaryOp::EqEq {
        "expected '===' and instead saw '=='."
      } else {
        "expected '!==' and instead saw '!='."
      };
      self
        .context
        .add_diagnostic(bin_expr.span, "eqeqeq", message)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn eqeqeq_test() {
    test_lint(
      "eqeqeq",
      "kumiko == oumae",
      vec![Eqeqeq::new()],
      json!([{
        "code": "eqeqeq",
        "message": "expected '===' and instead saw '=='.",
        "location": {
          "filename": "eqeqeq",
          "line": 1,
          "col": 0
        }
      }]),
    );

    test_lint(
      "eqeqeq",
      "reina != kousaka",
      vec![Eqeqeq::new()],
      json!([{
        "code": "eqeqeq",
        "message": "expected '!==' and instead saw '!='.",
        "location": {
          "filename": "eqeqeq",
          "line": 1,
          "col": 0
        }
      }]),
    );

    test_lint("eqeqeq", "midori == null", vec![Eqeqeq::new()], json!([]));

    test_lint("eqeqeq", "null == hazuki", vec![Eqeqeq::new()], json!([]));
  }
}
