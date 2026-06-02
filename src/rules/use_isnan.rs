// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{
  BinaryExpression, BinaryOperator, Expression, IdentifierReference, Program,
  SwitchStatement,
};
use derive_more::Display;

#[derive(Debug)]
pub struct UseIsNaN;

const CODE: &str = "use-isnan";

#[derive(Display)]
enum UseIsNaNMessage {
  #[display(fmt = "Use the isNaN function to compare with NaN")]
  Comparison,

  #[display(
    fmt = "'switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch"
  )]
  SwitchUnmatched,

  #[display(
    fmt = "'case NaN' can never match. Use Number.isNaN before the switch"
  )]
  CaseUnmatched,
}

impl LintRule for UseIsNaN {
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
    let mut handler = UseIsNaNHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct UseIsNaNHandler;

fn is_nan_identifier(ident: &IdentifierReference) -> bool {
  ident.name == "NaN"
}

impl Handler<'_> for UseIsNaNHandler {
  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    ctx: &mut Context,
  ) {
    if bin_expr.operator == BinaryOperator::Equality
      || bin_expr.operator == BinaryOperator::Inequality
      || bin_expr.operator == BinaryOperator::StrictEquality
      || bin_expr.operator == BinaryOperator::StrictInequality
      || bin_expr.operator == BinaryOperator::LessThan
      || bin_expr.operator == BinaryOperator::LessEqualThan
      || bin_expr.operator == BinaryOperator::GreaterThan
      || bin_expr.operator == BinaryOperator::GreaterEqualThan
    {
      if let Expression::Identifier(ident) = &bin_expr.left {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(bin_expr.span, CODE, UseIsNaNMessage::Comparison);
        }
      }
      if let Expression::Identifier(ident) = &bin_expr.right {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(bin_expr.span, CODE, UseIsNaNMessage::Comparison);
        }
      }
    }
  }

  fn switch_statement(
    &mut self,
    switch_stmt: &SwitchStatement,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &switch_stmt.discriminant {
      if is_nan_identifier(ident) {
        ctx.add_diagnostic(
          switch_stmt.span,
          CODE,
          UseIsNaNMessage::SwitchUnmatched,
        );
      }
    }

    for case in &switch_stmt.cases {
      if let Some(Expression::Identifier(ident)) = &case.test {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(case.span, CODE, UseIsNaNMessage::CaseUnmatched);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn use_isnan_valid() {
    assert_lint_ok! {
      UseIsNaN,
      "var x = NaN;",
      "isNaN(NaN) === true;",
      "isNaN(123) !== true;",
      "Number.isNaN(NaN) === true;",
      "Number.isNaN(123) !== true;",
      "foo(NaN + 1);",
      "foo(1 + NaN);",
      "foo(NaN - 1)",
      "foo(1 - NaN)",
      "foo(NaN * 2)",
      "foo(2 * NaN)",
      "foo(NaN / 2)",
      "foo(2 / NaN)",
      "var x; if (x = NaN) { }",
      "var x = Number.NaN;",
      "isNaN(Number.NaN) === true;",
      "Number.isNaN(Number.NaN) === true;",
      "foo(Number.NaN + 1);",
      "foo(1 + Number.NaN);",
      "foo(Number.NaN - 1)",
      "foo(1 - Number.NaN)",
      "foo(Number.NaN * 2)",
      "foo(2 * Number.NaN)",
      "foo(Number.NaN / 2)",
      "foo(2 / Number.NaN)",
      "var x; if (x = Number.NaN) { }",
      "x === Number[NaN];",
    };
  }

  #[test]
  fn use_isnan_invalid() {
    assert_lint_err! {
      UseIsNaN,
      "42 === NaN": [
      {
        col: 0,
        message: UseIsNaNMessage::Comparison,
      }],
      r#"
switch (NaN) {
  case NaN:
    break;
  default:
    break;
}
        "#: [
      {
        line: 2,
        col: 0,
        message: UseIsNaNMessage::SwitchUnmatched,
      },
      {
        line: 3,
        col: 2,
        message: UseIsNaNMessage::CaseUnmatched,
      }],
    }
  }
}
