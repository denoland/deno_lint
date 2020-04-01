// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::CallExpr;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprOrSuper;

pub struct NoEval {
  context: Context,
}

impl NoEval {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoEval {
  fn walk_call_expr(&self, call_expr: CallExpr) {
    if let ExprOrSuper::Expr(expr) = call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        if ident.sym.to_string() == "eval" {
          self.context.add_diagnostic(
            &call_expr.span,
            "noEval",
            "`eval` call is not allowed",
          );
        }
      }
    }
  }
}
