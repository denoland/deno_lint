// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{Expr, ThrowStmt};
use deno_ast::SourceRanged;
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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoThrowLiteralHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_throw_literal.md")
  }
}

struct NoThrowLiteralHandler;

impl Handler for NoThrowLiteralHandler {
  fn throw_stmt(&mut self, throw_stmt: &ThrowStmt, ctx: &mut Context) {
    match throw_stmt.arg {
      Expr::Lit(_) => ctx.add_diagnostic(
        throw_stmt.range(),
        CODE,
        NoThrowLiteralMessage::ErrObjectExpected,
      ),
      Expr::Ident(ident) if *ident.sym() == *"undefined" => ctx.add_diagnostic(
        throw_stmt.range(),
        CODE,
        NoThrowLiteralMessage::Undefined,
      ),
      _ => {}
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
