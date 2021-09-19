// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view::{Expr, NewExpr, ParenExpr};
use std::sync::Arc;

#[derive(Debug)]
pub struct NoAsyncPromiseExecutor;

const CODE: &str = "no-async-promise-executor";
const MESSAGE: &str = "Async promise executors are not allowed";
const HINT: &str =
  "Remove `async` from executor function and adjust promise code as needed";

impl LintRule for NoAsyncPromiseExecutor {
  fn new() -> Arc<Self> {
    Arc::new(NoAsyncPromiseExecutor)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    _context: &mut Context<'view>,
    _program: ProgramRef<'view>,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoAsyncPromiseExecutorHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_async_promise_executor.md")
  }
}

fn is_async_function(expr: &Expr) -> bool {
  match expr {
    Expr::Fn(fn_expr) => fn_expr.function.is_async(),
    Expr::Arrow(arrow_expr) => arrow_expr.is_async(),
    Expr::Paren(ParenExpr { ref expr, .. }) => is_async_function(expr),
    _ => false,
  }
}

struct NoAsyncPromiseExecutorHandler;

impl Handler for NoAsyncPromiseExecutorHandler {
  fn new_expr(&mut self, new_expr: &NewExpr, context: &mut Context) {
    if let Expr::Ident(ident) = &new_expr.callee {
      let name = ident.inner.as_ref();
      if name != "Promise" {
        return;
      }

      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          if is_async_function(&first_arg.expr) {
            context.add_diagnostic_with_hint(
              new_expr.span(),
              CODE,
              MESSAGE,
              HINT,
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

  #[test]
  fn no_async_promise_executor_valid() {
    assert_lint_ok! {
      NoAsyncPromiseExecutor,
      "new Promise(function(resolve, reject) {});",
      "new Promise((resolve, reject) => {});",
      "new Promise((resolve, reject) => {}, async function unrelated() {})",
      "new Foo(async (resolve, reject) => {})",
      "new class { foo() { new Promise(function(resolve, reject) {}); } }",
    };
  }

  #[test]
  fn no_async_promise_executor_invalid() {
    assert_lint_err! {
      NoAsyncPromiseExecutor,
      "new Promise(async function(resolve, reject) {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(async function foo(resolve, reject) {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(async (resolve, reject) => {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(((((async () => {})))));": [{ col: 0, message: MESSAGE, hint: HINT }],
      // nested
      r#"
const a = new class {
  foo() {
    let b = new Promise(async function(resolve, reject) {});
  }
}
      "#: [{ line: 4, col: 12, message: MESSAGE, hint: HINT }],
    }
  }
}
