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
      let name = ident.sym.to_string();
      if name != "Promise" {
        return;
      }

      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          let is_async = match &*first_arg.expr {
            Expr::Fn(fn_expr) => fn_expr.function.is_async,
            Expr::Arrow(arrow_expr) => arrow_expr.is_async,
            _ => return,
          };

          if is_async {
            self.context.add_diagnostic(
              new_expr.span,
              "noAsyncPromiseExecutor",
              "Async promise executors are not allowed",
            );
          }
        }
      }
    }
  }
}
