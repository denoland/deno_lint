// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::CallExpr;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprOrSuper;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEval {
  context: Context,
}

impl NoEval {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEval {
  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
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
