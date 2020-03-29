// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::Expr;
use swc_ecma_ast::UnaryExpr;
use swc_ecma_ast::UnaryOp;

pub struct NoDeleteVar {
  context: Context,
}

impl NoDeleteVar {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoDeleteVar {
  fn walk_unary_expr(&self, unary_expr: UnaryExpr) {
    if unary_expr.op != UnaryOp::Delete {
      return;
    }

    match *unary_expr.arg {
      Expr::Ident(_) => {
        self.context.add_diagnostic(
          &unary_expr.span,
          "noDeleteVar",
          "Variables shouldn't be deleted",
        );
      }
      _ => {}
    }
  }
}
