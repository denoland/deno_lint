// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;

#[derive(Debug)]
pub struct ValidTypeof;

const CODE: &str = "valid-typeof";
const MESSAGE: &str = "Invalid typeof comparison value";

impl LintRule for ValidTypeof {
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
    let mut handler = ValidTypeofHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }

  fn priority(&self) -> u32 {
    0
  }
}

struct ValidTypeofHandler;

impl Handler<'_> for ValidTypeofHandler {
  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    ctx: &mut Context,
  ) {
    if !matches!(
      bin_expr.operator,
      BinaryOperator::Equality
        | BinaryOperator::Inequality
        | BinaryOperator::StrictEquality
        | BinaryOperator::StrictInequality
    ) {
      return;
    }

    let (typeof_expr, operand) = match (&bin_expr.left, &bin_expr.right) {
      (Expression::UnaryExpression(unary), operand)
        if unary.operator == UnaryOperator::Typeof =>
      {
        (unary, operand)
      }
      (operand, Expression::UnaryExpression(unary))
        if unary.operator == UnaryOperator::Typeof =>
      {
        (unary, operand)
      }
      _ => return,
    };

    let _ = typeof_expr;

    match operand {
      Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::Typeof => {}
      Expression::StringLiteral(str_lit) => {
        if !is_valid_typeof_string(str_lit.value.as_str()) {
          ctx.add_diagnostic(str_lit.span, CODE, MESSAGE);
        }
      }
      Expression::TemplateLiteral(tpl) => {
        if tpl
          .string_repr()
          .is_some_and(|s| !is_valid_typeof_string(&s))
        {
          ctx.add_diagnostic(tpl.span, CODE, MESSAGE);
        }
      }
      Expression::Identifier(ident) if ident.name.as_str() == "undefined" => {
        ctx.add_diagnostic(ident.span, CODE, MESSAGE);
      }
      Expression::BooleanLiteral(_)
      | Expression::NullLiteral(_)
      | Expression::NumericLiteral(_)
      | Expression::BigIntLiteral(_)
      | Expression::RegExpLiteral(_) => {
        ctx.add_diagnostic(operand.span(), CODE, MESSAGE);
      }
      _ => {}
    }
  }
}

fn is_valid_typeof_string(str: &str) -> bool {
  matches!(
    str,
    "undefined"
      | "object"
      | "boolean"
      | "number"
      | "string"
      | "function"
      | "symbol"
      | "bigint"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_typeof_valid() {
    assert_lint_ok! {
      ValidTypeof,
      r#"typeof foo === "undefined""#,
      r#"typeof foo === "object""#,
      r#"typeof foo === "boolean""#,
      r#"typeof foo === "number""#,
      r#"typeof foo === "string""#,
      r#"typeof foo === "function""#,
      r#"typeof foo === "symbol""#,
      r#"typeof foo === "bigint""#,

      r#"typeof foo == 'undefined'"#,
      r#"typeof foo == 'object'"#,
      r#"typeof foo == 'boolean'"#,
      r#"typeof foo == 'number'"#,
      r#"typeof foo == 'string'"#,
      r#"typeof foo == 'function'"#,
      r#"typeof foo == 'symbol'"#,
      r#"typeof foo == 'bigint'"#,

      // https://github.com/denoland/deno_lint/issues/741
      r#"typeof foo !== `undefined`"#,
      r#"typeof foo !== `object`"#,
      r#"typeof foo !== `boolean`"#,
      r#"typeof foo !== `number`"#,
      r#"typeof foo !== `string`"#,
      r#"typeof foo !== `function`"#,
      r#"typeof foo !== `symbol`"#,
      r#"typeof foo !== `bigint`"#,

      r#"typeof bar != typeof qux"#,
      r#"const type = "string"; typeof foo === type"#,
      r#"typeof bar == Object"#,
      r#"typeof baz === anotherVariable"#,
      r#"typeof foo === obj.type"#,
      r#"typeof foo === getExpected()"#,
    };
  }

  #[test]
  fn valid_typeof_invalid() {
    assert_lint_err! {
      ValidTypeof,
      r#"typeof foo === "strnig""#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo == "undefimed""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar != "nunber""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar !== "fucntion""#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo === undefined"#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo === null"#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo === 5"#: [{
        col: 15,
        message: MESSAGE
      }],
    }
  }
}
