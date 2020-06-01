// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Expr;
use swc_ecma_ast::NewExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoAsyncPromiseExecutor;

impl LintRule for NoAsyncPromiseExecutor {
  fn new() -> Box<Self> {
    Box::new(NoAsyncPromiseExecutor)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoAsyncPromiseExecutorVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoAsyncPromiseExecutorVisitor {
  context: Context,
}

impl NoAsyncPromiseExecutorVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoAsyncPromiseExecutorVisitor {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_async_promise_executor_test() {
    assert_lint_ok_n::<NoAsyncPromiseExecutor>(vec![
      "new Promise(function(a, b) {});",
      "new Promise((a, b) => {});",
    ]);
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async function(a, b) {});",
      "noAsyncPromiseExecutor",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async (a, b) => {});",
      "noAsyncPromiseExecutor",
      0,
    );
  }
}
