// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::Expr;
use swc_ecma_ast::UnaryExpr;
use swc_ecma_ast::UnaryOp;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDeleteVar {
  context: Context,
}

impl NoDeleteVar {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDeleteVar {
  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _parent: &dyn Node) {
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
