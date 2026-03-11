// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoNonNullAssertion;

const CODE: &str = "no-non-null-assertion";

#[derive(Display)]
enum NoNonNullAssertionMessage {
  #[display(fmt = "do not use non-null assertion")]
  Unexpected,
}

impl LintRule for NoNonNullAssertion {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoNonNullAssertionHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoNonNullAssertionHandler;

impl Handler<'_> for NoNonNullAssertionHandler {
  fn ts_non_null_expression(
    &mut self,
    non_null_expr: &TSNonNullExpression,
    ctx: &mut Context,
  ) {
    // In OXC we can't check parent, but the original logic was:
    // report only if the parent is NOT also a TSNonNullExpression.
    // The innermost non-null in a chain like x!! is itself a TSNonNullExpression
    // whose expression is also a TSNonNullExpression. So we report only when the
    // inner expression is NOT a TSNonNullExpression (i.e. we report the outermost one).
    if !matches!(
      &non_null_expr.expression,
      Expression::TSNonNullExpression(_)
    ) {
      ctx.add_diagnostic(
        non_null_expr.span,
        CODE,
        NoNonNullAssertionMessage::Unexpected,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_non_null_assertion_valid() {
    assert_lint_ok! {
      NoNonNullAssertion,
      "instance.doWork();",
      "foo.bar?.includes('baz')",
      "x;",
      "x.y;",
      "x.y.z;",
      "x?.y.z;",
      "x?.y?.z;",
      "!x;",
    };
  }

  #[test]
  fn no_non_null_assertion_invalid() {
    assert_lint_err! {
      NoNonNullAssertion,

      r#"instance!.doWork()"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"foo.bar!.includes('baz');"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y.z!?.();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!?.y.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!?.[y].z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y.z!!();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y!!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!!.y;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!!!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y?.z!();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y.z!();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x![y]?.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x![y];"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"!x!.y;"#: [
      {
        col: 1,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!.y?.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!.y;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],

    }
  }
}
