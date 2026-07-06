// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct NoAwaitInSyncFn;

const CODE: &str = "no-await-in-sync-fn";
const MESSAGE: &str = "Unexpected `await` inside a non-async function.";
const HINT: &str = "Remove `await` in the function body or change the function to an async function.";

impl LintRule for NoAwaitInSyncFn {
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
    let mut handler = NoAwaitInSyncFnHandler {
      is_async_stack: vec![],
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoAwaitInSyncFnHandler {
  /// Stack tracking whether each function scope is async.
  is_async_stack: Vec<bool>,
}

impl Handler<'_> for NoAwaitInSyncFnHandler {
  fn function(&mut self, function: &Function, _ctx: &mut Context) {
    match function.r#type {
      FunctionType::FunctionDeclaration | FunctionType::FunctionExpression => {}
      _ => return,
    }
    self.is_async_stack.push(function.r#async);
  }

  fn function_exit(&mut self, function: &Function, _ctx: &mut Context) {
    match function.r#type {
      FunctionType::FunctionDeclaration | FunctionType::FunctionExpression => {}
      _ => return,
    }
    self.is_async_stack.pop();
  }

  fn arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self.is_async_stack.push(arrow.r#async);
  }

  fn arrow_function_expression_exit(
    &mut self,
    _arrow: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self.is_async_stack.pop();
  }

  fn await_expression(
    &mut self,
    await_expr: &AwaitExpression,
    ctx: &mut Context,
  ) {
    if let Some(&is_async) = self.is_async_stack.last() {
      if !is_async {
        ctx.add_diagnostic_with_hint(await_expr.span, CODE, MESSAGE, HINT);
      }
    }
    // If stack is empty, we're at top-level; top-level await is fine.
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_await_in_sync_fn_valid() {
    assert_lint_ok! {
      NoAwaitInSyncFn,
      r#"
      async function foo(things) {
        await bar();
      }
      "#,
      r#"
      const foo = async (things) => {
        await bar();
      }
      "#,
      r#"
      const foo = async function(things) {
        await bar();
      }
      "#,
      r#"
      const foo = {
        async foo(things) {
          await bar();
        }
      }
      "#,
      r#"
      class Foo {
        async foo(things) {
          await bar();
        }
      }
      "#,
      r#"
      class Foo {
        async #foo(things) {
          await bar();
        }
      }
      "#,
    }
  }

  #[test]
  fn no_await_in_sync_fn_invalid() {
    assert_lint_err! {
      NoAwaitInSyncFn,
      MESSAGE,
      HINT,
      r#"
      function foo(things) {
        await bar();
      }
      "#: [{ line: 3, col: 8 }],
      r#"
      const foo = things => {
        await bar();
      }
      "#: [{ line: 3, col: 8 }],
      r#"
      const foo = function (things) {
        await bar();
      }
      "#: [{ line: 3, col: 8 }],
      r#"
      const foo = {
        foo(things) {
          await bar();
        }
      }
      "#: [{ line: 4, col: 10 }],
      r#"
      class Foo {
        foo(things) {
          await bar();
        }
      }
      "#: [{ line: 4, col: 10 }],
      r#"
      class Foo {
        #foo(things) {
          await bar();
        }
      }
      "#: [{ line: 4, col: 10 }],
    }
  }
}
