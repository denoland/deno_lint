// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{AwaitExpr, Expr};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoUnnecessaryAwait;

const CODE: &str = "no-unnecessary-await";

#[derive(Display)]
enum NoUnnecessaryAwaitMessage {
  #[display(fmt = "Unexpected `await` on a non-Promise value")]
  Unexpected,
}

#[derive(Display)]
enum NoUnnecessaryAwaitHint {
  #[display(fmt = "Consider removing the `await`")]
  Remove,
}

impl LintRule for NoUnnecessaryAwait {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoUnnecessaryAwaitHandler.traverse(program, context);
  }
}

struct NoUnnecessaryAwaitHandler;

impl Handler for NoUnnecessaryAwaitHandler {
  fn await_expr(&mut self, await_expr: &AwaitExpr, ctx: &mut Context) {
    if not_promise(&await_expr.arg) {
      ctx.add_diagnostic_with_hint(
        await_expr.range(),
        CODE,
        NoUnnecessaryAwaitMessage::Unexpected,
        NoUnnecessaryAwaitHint::Remove,
      );
    }
  }
}

/// Returns `true` for expressions that are definitely not thenables/promises,
/// mirroring oxc's `not_promise`.
fn not_promise(expr: &Expr) -> bool {
  match expr {
    Expr::Array(_)
    | Expr::Arrow(_)
    | Expr::Await(_)
    | Expr::Bin(_)
    | Expr::Class(_)
    | Expr::Fn(_)
    | Expr::JSXElement(_)
    | Expr::JSXFragment(_)
    | Expr::Lit(_)
    | Expr::Tpl(_)
    | Expr::Unary(_)
    | Expr::Update(_) => true,
    Expr::Seq(seq) => seq.exprs.last().is_some_and(not_promise),
    Expr::Paren(paren) => not_promise(&paren.expr),
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_unnecessary_await.rs
  // MIT Licensed.

  #[test]
  fn no_unnecessary_await_valid() {
    assert_lint_ok! {
      NoUnnecessaryAwait,
      "await {then}",
      "await a ? b : c",
      "await a || b",
      "await a && b",
      "await a ?? b",
      "await new Foo()",
      "await tagged``",
      "class A { async foo() { await this }}",
      "async function * foo() {await (yield bar);}",
      "await (1, Promise.resolve())",
    };
  }

  #[test]
  fn no_unnecessary_await_invalid() {
    assert_lint_err! {
      NoUnnecessaryAwait,
      "await []": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await [Promise.resolve()]": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (() => {})": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (() => Promise.resolve())": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (a === b)": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (a instanceof Promise)": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (a > b)": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await class {}": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await class extends Promise {}": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await function() {}": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await function name() {}": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await function() { return Promise.resolve() }": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await 0": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await 1": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await \"\"": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await \"string\"": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await true": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await false": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await null": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await 0n": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await 1n": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await `${Promise.resolve()}`": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await !Promise.resolve()": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await void Promise.resolve()": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await +Promise.resolve()": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await ~1": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await ++foo": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await foo--": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await (Promise.resolve(), 1)": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "async function foo() {+await +1}": [
        { col: 23, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "async function foo() {-await-1}": [
        { col: 23, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "async function foo() {+await -1}": [
        { col: 23, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "await await this.assertTotalDocumentCount(expectedFormattedTotalDocCount);": [
        { col: 0, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "async function foo() {+await ++a}": [
        { col: 23, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ],
      "async function foo() {-await --a}": [
        { col: 23, message: NoUnnecessaryAwaitMessage::Unexpected, hint: NoUnnecessaryAwaitHint::Remove }
      ]
    };
  }
}
