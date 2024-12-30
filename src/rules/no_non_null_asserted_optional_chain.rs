// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{Callee, Expr, TsNonNullExpr};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoNonNullAssertedOptionalChain;

const CODE: &str = "no-non-null-asserted-optional-chain";

#[derive(Display)]
enum NoNonNullAssertedOptionalChainMessage {
  #[display(
    fmt = "Optional chain expressions can return undefined by design - using a non-null assertion is unsafe and wrong."
  )]
  WrongAssertion,
}

impl LintRule for NoNonNullAssertedOptionalChain {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoNonNullAssertedOptionalChainHandler.traverse(program, context);
  }
}

struct NoNonNullAssertedOptionalChainHandler;

fn check_expr_for_nested_optional_assert(
  range: SourceRange,
  expr: &Expr,
  ctx: &mut Context,
) {
  if let Expr::OptChain(_) = expr {
    ctx.add_diagnostic(
      range,
      CODE,
      NoNonNullAssertedOptionalChainMessage::WrongAssertion,
    );
  }
}

impl Handler for NoNonNullAssertedOptionalChainHandler {
  fn ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &TsNonNullExpr,
    ctx: &mut Context,
  ) {
    match ts_non_null_expr.expr {
      Expr::Member(member_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.range(),
          &member_expr.obj,
          ctx,
        );
      }
      Expr::Call(call_expr) => {
        if let Callee::Expr(expr) = &call_expr.callee {
          check_expr_for_nested_optional_assert(
            ts_non_null_expr.range(),
            expr,
            ctx,
          );
        }
      }
      Expr::Paren(paren_expr) => check_expr_for_nested_optional_assert(
        ts_non_null_expr.range(),
        &paren_expr.expr,
        ctx,
      ),
      _ => {}
    };

    check_expr_for_nested_optional_assert(
      ts_non_null_expr.range(),
      &ts_non_null_expr.expr,
      ctx,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_non_null_asserted_optional_chain_valid() {
    assert_lint_ok! {
      NoNonNullAssertedOptionalChain,
      "foo.bar!;",
      "foo.bar()!;",
      "foo?.bar();",
      "foo?.bar;",
      "(foo?.bar).baz!;",
      "(foo?.bar()).baz!;",
    };
  }

  #[test]
  fn no_non_null_asserted_optional_chain_invalid() {
    assert_lint_err! {
      NoNonNullAssertedOptionalChain,
      r#"foo?.bar!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo?.['bar']!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo?.bar()!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo.bar?.()!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!.baz"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!().baz"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!()"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar!)"#: [
      {
        col: 1,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar!)()"#: [
      {
        col: 1,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
    }
  }
}
