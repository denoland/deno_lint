// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::Expr;
use swc_ecma_ast::NewExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoAsyncPromiseExecutor {
  context: Context,
}

impl NoAsyncPromiseExecutor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoAsyncPromiseExecutor {
  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if ident.sym.to_string() != "Promise" {
        return;
      }

      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          if let Expr::Fn(fn_expr) = &*first_arg.expr {
            if fn_expr.function.is_async {
              self.context.add_diagnostic(
                &new_expr.span,
                "noAsyncPromiseExecutor",
                "Async promise executors are not allowed",
              );
            }
          }
        }
      }
    }
  }
}
