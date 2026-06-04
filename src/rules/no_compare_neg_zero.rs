// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoCompareNegZero;

const CODE: &str = "no-compare-neg-zero";

#[derive(Display)]
enum NoCompareNegZeroMessage {
  #[display(fmt = "Do not compare against -0")]
  Unexpected,
}

#[derive(Display)]
enum NoCompareNegZeroHint {
  #[display(fmt = "Use Object.is(x, -0) instead")]
  ObjectIs,
}

impl LintRule for NoCompareNegZero {
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
    let mut handler = NoCompareNegZeroHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoCompareNegZeroHandler;

impl Handler<'_> for NoCompareNegZeroHandler {
  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    context: &mut Context,
  ) {
    if !bin_expr.operator.is_comparator() {
      return;
    }

    if is_neg_zero(&bin_expr.left) || is_neg_zero(&bin_expr.right) {
      context.add_diagnostic_with_hint(
        bin_expr.span,
        CODE,
        NoCompareNegZeroMessage::Unexpected,
        NoCompareNegZeroHint::ObjectIs,
      );
    }
  }
}

trait Comparator {
  fn is_comparator(&self) -> bool;
}

impl Comparator for BinaryOperator {
  fn is_comparator(&self) -> bool {
    matches!(
      self,
      BinaryOperator::Equality
        | BinaryOperator::Inequality
        | BinaryOperator::StrictEquality
        | BinaryOperator::StrictInequality
        | BinaryOperator::LessThan
        | BinaryOperator::LessEqualThan
        | BinaryOperator::GreaterThan
        | BinaryOperator::GreaterEqualThan
    )
  }
}

fn is_neg_zero(expr: &Expression) -> bool {
  if let Expression::UnaryExpression(unary) = expr {
    if unary.operator == UnaryOperator::UnaryNegation {
      if let Expression::NumericLiteral(num) = &unary.argument {
        return num.value == 0.0;
      }
    }
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-compare-neg-zero.js
  // MIT Licensed.

  #[test]
  fn no_compare_neg_zero_valid() {
    assert_lint_ok! {
      NoCompareNegZero,
      r#"if (x === 0) { }"#,
      r#"if (Object.is(x, -0)) { }"#,
      r#"x === 0"#,
      r#"0 === x"#,
      r#"x == 0"#,
      r#"0 == x"#,
      r#"x === '0'"#,
      r#"'0' === x"#,
      r#"x == '0'"#,
      r#"'0' == x"#,
      r#"x === '-0'"#,
      r#"'-0' === x"#,
      r#"x == '-0'"#,
      r#"'-0' == x"#,
      r#"x === -1"#,
      r#"-1 === x"#,
      r#"x < 0"#,
      r#"0 < x"#,
      r#"x <= 0"#,
      r#"0 <= x"#,
      r#"x > 0"#,
      r#"0 > x"#,
      r#"x >= 0"#,
      r#"0 >= x"#,
      r#"x != 0"#,
      r#"0 != x"#,
      r#"x !== 0"#,
      r#"0 !== x"#,
      r#"({}) == { foo: x === 0 }"#,
    };
  }

  #[test]
  fn no_compare_neg_zero_invalid() {
    assert_lint_err! {
      NoCompareNegZero,
      "if (x == -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x != -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 != x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x === -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x !== -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 !== x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x < -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 < x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x <= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 <= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x > -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 > x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x >= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 >= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x == -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0.0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x === -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0.0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],

      // nested
      "({}) == { foo: x === -0 }": [
        {
          col: 15,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ]
    };
  }
}
