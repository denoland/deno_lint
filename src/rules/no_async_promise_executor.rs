// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::NewExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoAsyncPromiseExecutor;

impl LintRule for NoAsyncPromiseExecutor {
  fn new() -> Box<Self> {
    Box::new(NoAsyncPromiseExecutor)
  }

  fn code(&self) -> &'static str {
    "no-async-promise-executor"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoAsyncPromiseExecutorVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoAsyncPromiseExecutorVisitor {
  context: Arc<Context>,
}

impl NoAsyncPromiseExecutorVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoAsyncPromiseExecutorVisitor {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.as_ref();
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
              "no-async-promise-executor",
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
  fn no_async_promise_executor_valid() {
    assert_lint_ok_n::<NoAsyncPromiseExecutor>(vec![
      "new Promise(function(resolve, reject) {});",
      "new Promise((resolve, reject) => {});",
      "new Promise((resolve, reject) => {}, async function unrelated() {})",
      "new Foo(async (resolve, reject) => {})",
      "new class { foo() { new Promise(async function(resolve, reject) {}); } }"
    ]);
  }

  #[test]
  fn no_async_promise_executor_invalid() {
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async function(resolve, reject) {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async function foo(resolve, reject) {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async (resolve, reject) => {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(((((async () => {})))));",
      0,
    );
    // nested
    assert_lint_err_on_line::<NoAsyncPromiseExecutor>(
      r#"
const a = new class {
  foo() {
    let b = new Promise(async function(resolve, reject) {});
  }
}
      "#,
      4,
      12,
    );
  }
}
