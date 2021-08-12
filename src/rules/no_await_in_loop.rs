// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, Program, ProgramRef};
use crate::handler::{Handler, Traverse};
use ast_view::NodeTrait;
use swc_common::Spanned;

pub struct NoAwaitInLoop;

const CODE: &str = "no-await-in-loop";
const MESSAGE: &str = "Unexpected `await` inside a loop.";
const HINT: &str = "Remove `await` in loop body, store all promises generated and then `await Promise.all(storedPromises)` after the loop";

impl LintRule for NoAwaitInLoop {
  fn new() -> Box<Self> {
    Box::new(NoAwaitInLoop)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoAwaitInLoopHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_await_in_loop.md")
  }
}

struct NoAwaitInLoopHandler;

impl Handler for NoAwaitInLoopHandler {
  fn await_expr(
    &mut self,
    await_expr: &ast_view::AwaitExpr,
    ctx: &mut Context,
  ) {
    fn inside_loop(
      await_expr: &ast_view::AwaitExpr,
      node: ast_view::Node,
    ) -> bool {
      use ast_view::Node::*;
      match node {
        FnDecl(_) | FnExpr(_) | ArrowExpr(_) => false,
        ForOfStmt(stmt) if stmt.await_token().is_some() => {
          // `await` is allowed to use within the body of `for await (const x of y) { ... }`
          false
        }
        ForInStmt(ast_view::ForInStmt { right, .. })
        | ForOfStmt(ast_view::ForOfStmt { right, .. }) => {
          // When it encounters `ForInStmt` or `ForOfStmt`, we should treat it as `inside_loop = true`
          // except for the case where the given `await_expr` is contained in the `right` part.
          // e.g. for (const x of await xs) { ... }
          //                      ^^^^^^^^ <-------- `right` part
          !right.span().contains(await_expr.span())
        }
        ForStmt(stmt) => {
          // When it encounters `ForStmt`, we should treat it as `inside_loop = true`
          // except for the case where the given `await_expr` is contained in the `init` part.
          // e.g. for (let i = await foo(); i < n; i++) { ... }
          //           ^^^^^^^^^^^^^^^^^^^ <---------- `init` part
          stmt
            .init
            .as_ref()
            .map_or(true, |init| !init.span().contains(await_expr.span()))
        }
        WhileStmt(_) | DoWhileStmt(_) => true,
        _ => {
          let parent = match node.parent() {
            Some(p) => p,
            None => return false,
          };
          inside_loop(await_expr, parent)
        }
      }
    }

    if inside_loop(await_expr, await_expr.as_node()) {
      ctx.add_diagnostic_with_hint(await_expr.span(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_await_in_loop_valid() {
    assert_lint_ok! {
      NoAwaitInLoop,
      r#"
async function foo(things) {
  const results = [];
  for (const thing of things) {
    results.push(bar(thing));
  }
  return baz(await Promise.all(results));
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    const a = async () => await bar(thing);
  }
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    async function a() {
      return await bar(42);
    }
  }
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    const a = async function() {
      return await bar(42);
    }
  }
}
      "#,
      r#"
async function foo(things) {
  for await (const thing of things) {
    console.log(await bar(thing));
  }
}
      "#,
      r#"
async function foo(things) {
  for await (const thing of await things) {
    console.log(await bar(thing));
  }
}
      "#,
      r#"
async function foo() {
  for (let i = await bar(); i < n; i++) {
    baz(i);
  }
}
      "#,
      r#"
async function foo() {
  for (const thing of await things) {
    bar(thing);
  }
}
      "#,
      r#"
async function foo() {
  for (let thing in await things) {
    bar(thing);
  }
}
      "#,
      r#"
function foo() {
  async function bar() {
    for (const thing of await things) {}
  }
}
      "#,

      // toplevel await
      r#"
for (const thing of things) {
  const a = async () => await bar(thing);
}
      "#,
      r#"
for (const thing of things) {
  async function a() {
    return await bar(42);
  }
}
      "#,
      r#"
for (const thing of things) {
  const a = async function() {
    return await bar(42);
  }
}
      "#,
      r#"
for await (const thing of things) {
  console.log(await bar(thing));
}
      "#,
      r#"
for await (const thing of await things) {
  console.log(await bar(thing));
}
      "#,
      r#"
for (let i = await bar(); i < n; i++) {
  baz(i);
}
      "#,
      r#"
for (const thing of await things) {
  bar(thing);
}
      "#,
      r#"
for (let thing in await things) {
  bar(thing);
}
      "#,
    };
  }

  #[test]
  fn no_await_in_loop_invalid() {
    assert_lint_err! {
      NoAwaitInLoop,
      r#"
async function foo(things) {
  const results = [];
  for (const thing of things) {
    results.push(await bar(thing));
  }
  return baz(results);
}
      "#: [{ line: 5, col: 17, message: MESSAGE, hint: HINT }],
      r#"
for (const thing of things) {
  results.push(await foo(thing));
}
      "#: [{ line: 3, col: 15, message: MESSAGE, hint: HINT }],
      r#"
for (let i = 0; i < await foo(); i++) {
  bar();
}
      "#: [{ line: 2, col: 20, message: MESSAGE, hint: HINT }],
      r#"
for (let i = 0; i < 42; await foo(i)) {
  bar();
}
      "#: [{ line: 2, col: 24, message: MESSAGE, hint: HINT }],
      r#"
for (let i = 0; i < 42; i++) {
  await bar();
}
      "#: [{ line: 3, col: 2, message: MESSAGE, hint: HINT }],
      r#"
for (const thing in things) {
  await foo(thing);
}
      "#: [{ line: 3, col: 2, message: MESSAGE, hint: HINT }],
      r#"
while (await foo()) {
  bar();
}
      "#: [{ line: 2, col: 7, message: MESSAGE, hint: HINT }],
      r#"
while (true) {
  await foo();
}
      "#: [{ line: 3, col: 2, message: MESSAGE, hint: HINT }],
      r#"
while (true) {
  await foo();
}
      "#: [{ line: 3, col: 2, message: MESSAGE, hint: HINT }],
      r#"
do {
  foo();
} while (await bar());
      "#: [{ line: 4, col: 9, message: MESSAGE, hint: HINT }],
      r#"
do {
  await foo();
} while (true);
      "#: [{ line: 3, col: 2, message: MESSAGE, hint: HINT }],
      r#"
for await (const thing of things) {
  async function foo() {
    for (const one of them) {
      await bar(one);
    }
  }
  await baz();
}
      "#: [{ line: 5, col: 6, message: MESSAGE, hint: HINT }],

      r#"
function foo() {
  async function bar() {
    for (const thing of things) {
      await baz(thing);
    }
  }
}
      "#: [{ line: 5, col: 6, message: MESSAGE, hint: HINT }],
      r#"
async function foo() {
  for (const thing of things) {
    const xs = bar(thing);
    for (const x in xs) {
      await baz(x);
    }
  }
}
      "#: [{ line: 6, col: 6, message: MESSAGE, hint: HINT }],
    }
  }
}
