// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoUnsafeNegation;

const CODE: &str = "no-unsafe-negation";

#[derive(Display)]
enum NoUnsafeNegationMessage {
  #[display(fmt = "Unexpected negating the left operand of `{}` operator", _0)]
  Unexpected(String),
}

const HINT: &str = "Add parentheses to clarify which range the negation operator should be applied to";

impl LintRule for NoUnsafeNegation {
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
    let mut handler = NoUnsafeNegationHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoUnsafeNegationHandler;

impl Handler<'_> for NoUnsafeNegationHandler {
  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    ctx: &mut Context,
  ) {
    if matches!(
      bin_expr.operator,
      BinaryOperator::In | BinaryOperator::Instanceof
    ) {
      if let Expression::UnaryExpression(unary_expr) = &bin_expr.left {
        if unary_expr.operator == UnaryOperator::LogicalNot {
          ctx.add_diagnostic_with_hint(
            bin_expr.span,
            CODE,
            NoUnsafeNegationMessage::Unexpected(
              bin_expr.operator.as_str().to_string(),
            ),
            HINT,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unsafe_negation_valid() {
    assert_lint_ok! {
      NoUnsafeNegation,
      "1 in [1, 2, 3]",
      "key in object",
      "foo instanceof Date",
      "!(1 in [1, 2, 3])",
      "!(key in object)",
      "!(foo instanceof Date)",
      "(!key) in object",
      "(!foo) instanceof Date",
    };
  }

  #[test]
  fn no_unsafe_negation_invalid() {
    assert_lint_err! {
      NoUnsafeNegation,
      "!1 in [1, 2, 3]": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "in"),
          hint: HINT
        }
      ],
      "!key in object": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "in"),
          hint: HINT
        }
      ],
      "!foo instanceof Date": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "instanceof"),
          hint: HINT
        }
      ],
    };
  }
}
