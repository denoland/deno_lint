// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct UseIsNaN;

impl LintRule for UseIsNaN {
  fn new() -> Box<Self> {
    Box::new(UseIsNaN)
  }

  fn code(&self) -> &'static str {
    "use-isnan"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = UseIsNaNVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct UseIsNaNVisitor {
  context: Arc<Context>,
}

impl UseIsNaNVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

fn is_nan_identifier(ident: &swc_ecma_ast::Ident) -> bool {
  ident.sym == swc_atoms::js_word!("NaN")
}

impl Visit for UseIsNaNVisitor {
  fn visit_bin_expr(
    &mut self,
    bin_expr: &swc_ecma_ast::BinExpr,
    _parent: &dyn Node,
  ) {
    if bin_expr.op == swc_ecma_ast::BinaryOp::EqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::NotEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::EqEqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::NotEqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::Lt
      || bin_expr.op == swc_ecma_ast::BinaryOp::LtEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::Gt
      || bin_expr.op == swc_ecma_ast::BinaryOp::GtEq
    {
      if let swc_ecma_ast::Expr::Ident(ident) = &*bin_expr.left {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            "use-isnan",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
      if let swc_ecma_ast::Expr::Ident(ident) = &*bin_expr.right {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            "use-isnan",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
    }
  }

  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecma_ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    if let swc_ecma_ast::Expr::Ident(ident) = &*switch_stmt.discriminant {
      if is_nan_identifier(&ident) {
        self.context.add_diagnostic(
          switch_stmt.span,
          "use-isnan",
          "'switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch",
        );
      }
    }

    for case in &switch_stmt.cases {
      if let Some(expr) = &case.test {
        if let swc_ecma_ast::Expr::Ident(ident) = &**expr {
          if is_nan_identifier(ident) {
            self.context.add_diagnostic(
              case.span,
              "use-isnan",
              "'case NaN' can never match. Use Number.isNaN before the switch",
            );
          }
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
  fn use_isnan_test() {
    assert_lint_err::<UseIsNaN>("42 === NaN", 0);
    assert_lint_err_on_line_n::<UseIsNaN>(
      r#"
switch (NaN) {
  case NaN:
    break;
  default:
    break;
}
      "#,
      vec![(2, 0), (3, 2)],
    );
  }
}
