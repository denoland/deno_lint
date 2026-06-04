// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{Expression, Program, ThrowStatement};
use derive_more::Display;

#[derive(Debug)]
pub struct NoThrowLiteral;

const CODE: &str = "no-throw-literal";

#[derive(Display)]
enum NoThrowLiteralMessage {
  #[display(fmt = "expected an error object to be thrown")]
  ErrObjectExpected,

  #[display(fmt = "do not throw undefined")]
  Undefined,
}

impl LintRule for NoThrowLiteral {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoThrowLiteralHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoThrowLiteralHandler;

fn is_literal(expr: &Expression) -> bool {
  matches!(
    expr,
    Expression::BooleanLiteral(_)
      | Expression::NullLiteral(_)
      | Expression::NumericLiteral(_)
      | Expression::BigIntLiteral(_)
      | Expression::RegExpLiteral(_)
      | Expression::StringLiteral(_)
  )
}

impl Handler<'_> for NoThrowLiteralHandler {
  fn throw_statement(
    &mut self,
    throw_stmt: &ThrowStatement,
    ctx: &mut Context,
  ) {
    if is_literal(&throw_stmt.argument) {
      ctx.add_diagnostic(
        throw_stmt.span,
        CODE,
        NoThrowLiteralMessage::ErrObjectExpected,
      );
    } else if let Expression::Identifier(ident) = &throw_stmt.argument {
      if ident.name == "undefined" {
        ctx.add_diagnostic(
          throw_stmt.span,
          CODE,
          NoThrowLiteralMessage::Undefined,
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_throw_literal_valid() {
    assert_lint_ok! {
      NoThrowLiteral,
      "throw e",
    };
  }

  #[test]
  fn no_throw_literal_invalid() {
    assert_lint_err! {
      NoThrowLiteral,
      r#"throw 'kumiko'"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw true"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw 1096"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw null"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw undefined"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::Undefined,
      }],
    }
  }
}
