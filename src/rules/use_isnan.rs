// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;

pub struct UseIsNaN {
  context: Context,
}

impl UseIsNaN {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

fn is_nan_identifier(ident: &swc_ecma_ast::Ident) -> bool {
  ident.sym == swc_atoms::js_word!("NaN")
}

impl AstTraverser for UseIsNaN {
  fn walk_bin_expr(&self, bin_expr: swc_ecma_ast::BinExpr) {
    if bin_expr.op == swc_ecma_ast::BinaryOp::EqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::NotEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::EqEqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::NotEqEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::Lt
      || bin_expr.op == swc_ecma_ast::BinaryOp::LtEq
      || bin_expr.op == swc_ecma_ast::BinaryOp::Gt
      || bin_expr.op == swc_ecma_ast::BinaryOp::GtEq
    {
      if let swc_ecma_ast::Expr::Ident(ident) = *bin_expr.left {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            &bin_expr.span,
            "useIsNaN",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
      if let swc_ecma_ast::Expr::Ident(ident) = *bin_expr.right {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            &bin_expr.span,
            "useIsNaN",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
    }
  }

  fn walk_switch_stmt(&self, switch_stmt: swc_ecma_ast::SwitchStmt) {
    if let swc_ecma_ast::Expr::Ident(ident) = *switch_stmt.discriminant {
      if is_nan_identifier(&ident) {
        self.context.add_diagnostic(
          &switch_stmt.span,
          "useIsNaN",
          "switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch",
        );
      }
    }

    for case in switch_stmt.cases {
      if let Some(expr) = case.test {
        if let swc_ecma_ast::Expr::Ident(ident) = *expr {
          if is_nan_identifier(&ident) {
            self.context.add_diagnostic(
              &case.span,
              "useIsNaN",
              "'case NaN' can never match. Use Number.isNaN before the switch",
            );
          }
        }
      }
    }
  }
}
