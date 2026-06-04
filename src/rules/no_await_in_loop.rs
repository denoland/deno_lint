// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, AwaitExpression, DoWhileStatement, ForInStatement,
  ForOfStatement, ForStatement, Function, Program, WhileStatement,
};
use deno_ast::oxc::span::{GetSpan, Span};

#[derive(Debug)]
pub struct NoAwaitInLoop;

const CODE: &str = "no-await-in-loop";
const MESSAGE: &str = "Unexpected `await` inside a loop.";
const HINT: &str = "Remove `await` in loop body, store all promises generated and then `await Promise.all(storedPromises)` after the loop";

impl LintRule for NoAwaitInLoop {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoAwaitInLoopHandler { scopes: vec![] };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

#[derive(Clone, Copy, Debug)]
enum ScopeKind {
  /// A loop body. `body_span` is the span of the body only (not the header/init/right).
  Loop { span: Span },
  /// A for-await-of loop (await is allowed inside its body).
  ForAwaitOf { span: Span },
  /// A function boundary (resets loop context).
  FunctionBoundary { span: Span },
}

struct NoAwaitInLoopHandler {
  scopes: Vec<ScopeKind>,
}

impl NoAwaitInLoopHandler {
  fn is_await_in_loop(&self, await_span: Span) -> bool {
    for scope in self.scopes.iter().rev() {
      match scope {
        ScopeKind::Loop { span } => {
          if span.start <= await_span.start && await_span.end <= span.end {
            return true;
          }
        }
        ScopeKind::ForAwaitOf { span } => {
          if span.start <= await_span.start && await_span.end <= span.end {
            // Inside a for-await-of, so this is OK
            return false;
          }
        }
        ScopeKind::FunctionBoundary { span } => {
          if span.start <= await_span.start && await_span.end <= span.end {
            // Inside a function boundary that contains the await.
            // This resets the loop context.
            return false;
          }
        }
      }
    }
    false
  }
}

impl Handler<'_> for NoAwaitInLoopHandler {
  fn await_expression(
    &mut self,
    await_expr: &AwaitExpression,
    ctx: &mut Context,
  ) {
    if self.is_await_in_loop(await_expr.span) {
      ctx.add_diagnostic_with_hint(await_expr.span, CODE, MESSAGE, HINT);
    }
  }

  fn for_statement(&mut self, node: &ForStatement, _ctx: &mut Context) {
    // For a ForStatement, the "loop" includes test, update, and body, but NOT init.
    // We use the body span for simplicity, but we also need to include test and update.
    // The body starts after init. We use the span from after the init to the end of the for.
    //
    // Structure: for (init; test; update) body
    // Awaits in test, update, and body are "in loop". Awaits in init are NOT.
    //
    // We compute a span that excludes init but includes everything else.
    let loop_start = if let Some(init) = &node.init {
      init.span().end
    } else {
      // No init, the loop span starts at the for statement's start
      node.span.start
    };
    self.scopes.push(ScopeKind::Loop {
      span: Span::new(loop_start, node.span.end),
    });
  }

  fn for_in_statement(&mut self, node: &ForInStatement, _ctx: &mut Context) {
    // For ForInStatement: `for (left in right) body`
    // Awaits in `right` are OK (not in loop), awaits in body are in loop.
    // We use the body span. But the body is the Statement after the right.
    // To exclude right, we start the loop span after the right expression.
    let loop_start = node.right.span().end;
    self.scopes.push(ScopeKind::Loop {
      span: Span::new(loop_start, node.span.end),
    });
  }

  fn for_of_statement(&mut self, node: &ForOfStatement, _ctx: &mut Context) {
    // For ForOfStatement: `for (left of right) body` or `for await (left of right) body`
    // Awaits in `right` are OK (not in loop).
    let loop_start = node.right.span().end;
    if node.r#await {
      self.scopes.push(ScopeKind::ForAwaitOf {
        span: Span::new(loop_start, node.span.end),
      });
    } else {
      self.scopes.push(ScopeKind::Loop {
        span: Span::new(loop_start, node.span.end),
      });
    }
  }

  fn while_statement(&mut self, node: &WhileStatement, _ctx: &mut Context) {
    // For while: `while (test) body`
    // Awaits in both test and body are in loop.
    self.scopes.push(ScopeKind::Loop { span: node.span });
  }

  fn do_while_statement(
    &mut self,
    node: &DoWhileStatement,
    _ctx: &mut Context,
  ) {
    // For do-while: `do body while (test)`
    // Awaits in both body and test are in loop.
    self.scopes.push(ScopeKind::Loop { span: node.span });
  }

  fn function(&mut self, node: &Function, _ctx: &mut Context) {
    self
      .scopes
      .push(ScopeKind::FunctionBoundary { span: node.span });
  }

  fn arrow_function_expression(
    &mut self,
    node: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self
      .scopes
      .push(ScopeKind::FunctionBoundary { span: node.span });
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
      MESSAGE,
      HINT,
      r#"
async function foo(things) {
  const results = [];
  for (const thing of things) {
    results.push(await bar(thing));
  }
  return baz(results);
}
      "#: [{ line: 5, col: 17 }],
      r#"
for (const thing of things) {
  results.push(await foo(thing));
}
      "#: [{ line: 3, col: 15 }],
      r#"
for (let i = 0; i < await foo(); i++) {
  bar();
}
      "#: [{ line: 2, col: 20 }],
      r#"
for (let i = 0; i < 42; await foo(i)) {
  bar();
}
      "#: [{ line: 2, col: 24 }],
      r#"
for (let i = 0; i < 42; i++) {
  await bar();
}
      "#: [{ line: 3, col: 2 }],
      r#"
for (const thing in things) {
  await foo(thing);
}
      "#: [{ line: 3, col: 2 }],
      r#"
while (await foo()) {
  bar();
}
      "#: [{ line: 2, col: 7 }],
      r#"
while (true) {
  await foo();
}
      "#: [{ line: 3, col: 2 }],
      r#"
while (true) {
  await foo();
}
      "#: [{ line: 3, col: 2 }],
      r#"
do {
  foo();
} while (await bar());
      "#: [{ line: 4, col: 9 }],
      r#"
do {
  await foo();
} while (true);
      "#: [{ line: 3, col: 2 }],
      r#"
for await (const thing of things) {
  async function foo() {
    for (const one of them) {
      await bar(one);
    }
  }
  await baz();
}
      "#: [{ line: 5, col: 6 }],

      r#"
function foo() {
  async function bar() {
    for (const thing of things) {
      await baz(thing);
    }
  }
}
      "#: [{ line: 5, col: 6 }],
      r#"
async function foo() {
  for (const thing of things) {
    const xs = bar(thing);
    for (const x in xs) {
      await baz(x);
    }
  }
}
      "#: [{ line: 6, col: 6 }]
    }
  }
}
