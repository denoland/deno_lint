// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::swc::ast::BinaryOp::*;
use deno_ast::swc::ast::Expr::Lit;
use deno_ast::swc::ast::Lit::Num;
use deno_ast::swc::ast::UnaryExpr;
use deno_ast::swc::ast::UnaryOp::Minus;
use deno_ast::view::{BinExpr, BinaryOp, Expr};
use deno_ast::SourceRanged;
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
  #[display(
    fmt = "Compare against 0 instead"
  )]
  CompareZeroInstead,
}

impl LintRule for NoCompareNegZero {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoCompareNegZeroHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_compare_neg_zero.md")
  }
}

struct NoCompareNegZeroHandler;

impl Handler for NoCompareNegZeroHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, context: &mut Context) {
    if !bin_expr.op().is_comparator() {
      return;
    }

    if bin_expr.left.is_neg_zero() || bin_expr.right.is_neg_zero() {
      context.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        NoCompareNegZeroMessage::Unexpected,
        NoCompareNegZeroHint::CompareZeroInstead,
      );
    }
  }
}

trait Comparator {
  fn is_comparator(&self) -> bool;
}

impl Comparator for BinaryOp {
  fn is_comparator(&self) -> bool {
    matches!(
      self,
      EqEq | NotEq | EqEqEq | NotEqEq | Lt | LtEq | Gt | GtEq
    )
  }
}

trait NegZero {
  fn is_neg_zero(&self) -> bool;
}

impl NegZero for Expr<'_> {
  fn is_neg_zero(&self) -> bool {
    match self {
      Expr::Unary(unary) => unary.inner.is_neg_zero(),
      _ => false,
    }
  }
}

impl NegZero for UnaryExpr {
  fn is_neg_zero(&self) -> bool {
    if let (Minus, Lit(Num(number))) = (self.op, &*self.arg) {
      return number.value == 0.0;
    }
    false
  }
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
      r#"{} == { foo: x === 0 }"#,
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
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x != -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 != x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x === -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x !== -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 !== x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x < -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 < x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x <= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 <= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x > -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 > x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x >= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0 >= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x == -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0.0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (x === -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],
      "if (-0.0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ],

      // nested
      "{} == { foo: x === -0 }": [
        {
          col: 13,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::CompareZeroInstead,
        }
      ]
    };
  }
}
