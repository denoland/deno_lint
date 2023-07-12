// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct NoAwaitInSyncFn;

const CODE: &str = "no-await-in-sync-fn";
const MESSAGE: &str = "Unexpected `await` inside a non-async function.";
const HINT: &str = "Remove `await` in the function body or change the function to an async function.";

impl LintRule for NoAwaitInSyncFn {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoAwaitInSyncFnHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_await_in_sync_fn.md")
  }
}

struct NoAwaitInSyncFnHandler;

impl Handler for NoAwaitInSyncFnHandler {
  fn await_expr(
    &mut self,
    await_expr: &ast_view::AwaitExpr,
    ctx: &mut Context,
  ) {
    fn inside_sync_fn(node: ast_view::Node) -> bool {
      use deno_ast::view::Node::*;
      match node {
        FnDecl(decl) => !decl.function.is_async(),
        FnExpr(decl) => !decl.function.is_async(),
        ArrowExpr(decl) => !decl.is_async(),
        _ => {
          let parent = match node.parent() {
            Some(p) => p,
            None => return false,
          };
          inside_sync_fn(parent)
        }
      }
    }

    if inside_sync_fn(await_expr.as_node()) {
      ctx.add_diagnostic_with_hint(await_expr.range(), CODE, MESSAGE, HINT);
    }
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
      class Foo {
        async foo(things) {
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
    }
  }
}
