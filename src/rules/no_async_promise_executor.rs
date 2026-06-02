// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct NoAsyncPromiseExecutor;

const CODE: &str = "no-async-promise-executor";
const MESSAGE: &str = "Async promise executors are not allowed";
const HINT: &str =
  "Remove `async` from executor function and adjust promise code as needed";

impl LintRule for NoAsyncPromiseExecutor {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoAsyncPromiseExecutorHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

fn is_async_function(expr: &Expression) -> bool {
  match expr {
    Expression::FunctionExpression(fn_expr) => fn_expr.r#async,
    Expression::ArrowFunctionExpression(arrow_expr) => arrow_expr.r#async,
    Expression::ParenthesizedExpression(paren) => {
      is_async_function(&paren.expression)
    }
    _ => false,
  }
}

fn is_async_argument(arg: &Argument) -> bool {
  match arg {
    Argument::FunctionExpression(fn_expr) => fn_expr.r#async,
    Argument::ArrowFunctionExpression(arrow_expr) => arrow_expr.r#async,
    Argument::ParenthesizedExpression(paren) => {
      is_async_function(&paren.expression)
    }
    _ => false,
  }
}

struct NoAsyncPromiseExecutorHandler;

impl Handler<'_> for NoAsyncPromiseExecutorHandler {
  fn new_expression(
    &mut self,
    new_expr: &NewExpression,
    context: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      let name = ident.name.as_str();
      if name != "Promise" {
        return;
      }

      if let Some(first_arg) = new_expr.arguments.first() {
        if is_async_argument(first_arg) {
          context.add_diagnostic_with_hint(new_expr.span, CODE, MESSAGE, HINT);
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
