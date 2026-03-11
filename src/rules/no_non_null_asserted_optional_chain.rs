// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{
  Expression, Program, TSNonNullExpression,
};
use deno_ast::oxc::span::Span;
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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoNonNullAssertedOptionalChainHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoNonNullAssertedOptionalChainHandler;

fn is_optional_chain(expr: &Expression) -> bool {
  // In OXC, optional chains are wrapped in ChainExpression.
  // Also check optional member/call expressions directly.
  match expr {
    Expression::ChainExpression(_) => true,
    Expression::StaticMemberExpression(m) => m.optional,
    Expression::ComputedMemberExpression(m) => m.optional,
    Expression::CallExpression(c) => c.optional,
    _ => false,
  }
}

fn check_expr_for_nested_optional_assert(
  span: Span,
  expr: &Expression,
  ctx: &mut Context,
) {
  if is_optional_chain(expr) {
    ctx.add_diagnostic(
      span,
      CODE,
      NoNonNullAssertedOptionalChainMessage::WrongAssertion,
    );
  }
}

impl Handler<'_> for NoNonNullAssertedOptionalChainHandler {
  fn ts_non_null_expression(
    &mut self,
    ts_non_null_expr: &TSNonNullExpression,
    ctx: &mut Context,
  ) {
    match &ts_non_null_expr.expression {
      Expression::StaticMemberExpression(member_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.span,
          &member_expr.object,
          ctx,
        );
      }
      Expression::ComputedMemberExpression(member_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.span,
          &member_expr.object,
          ctx,
        );
      }
      Expression::PrivateFieldExpression(member_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.span,
          &member_expr.object,
          ctx,
        );
      }
      Expression::CallExpression(call_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.span,
          &call_expr.callee,
          ctx,
        );
      }
      Expression::ParenthesizedExpression(paren_expr) => {
        check_expr_for_nested_optional_assert(
          ts_non_null_expr.span,
          &paren_expr.expression,
          ctx,
        );
      }
      _ => {}
    };

    check_expr_for_nested_optional_assert(
      ts_non_null_expr.span,
      &ts_non_null_expr.expression,
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
