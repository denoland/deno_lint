// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::BinaryOp;
use deno_ast::view::{BinExpr, Expr};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct BadObjectLiteralComparison;

const CODE: &str = "bad-object-literal-comparison";

#[derive(Display)]
enum BadObjectLiteralComparisonMessage {
  #[display(
    fmt = "Comparing against an empty object literal is always false (or always true for `!=`/`!==`); object references are never equal"
  )]
  Object,
  #[display(
    fmt = "Comparing against an empty array literal is always false (or always true for `!=`/`!==`); array references are never equal"
  )]
  Array,
}

#[derive(Display)]
enum BadObjectLiteralComparisonHint {
  #[display(
    fmt = "To check for an empty object, use `Object.keys(x).length === 0`"
  )]
  Object,
  #[display(
    fmt = "To check for an empty array, use `Array.isArray(x) && x.length === 0`"
  )]
  Array,
}

impl LintRule for BadObjectLiteralComparison {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    BadObjectLiteralComparisonHandler.traverse(program, context);
  }
}

struct BadObjectLiteralComparisonHandler;

impl Handler for BadObjectLiteralComparisonHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, ctx: &mut Context) {
    if !matches!(
      bin_expr.op(),
      BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
    ) {
      return;
    }

    if is_empty_object(&bin_expr.left) || is_empty_object(&bin_expr.right) {
      ctx.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        BadObjectLiteralComparisonMessage::Object,
        BadObjectLiteralComparisonHint::Object,
      );
    } else if is_empty_array(&bin_expr.left) || is_empty_array(&bin_expr.right)
    {
      ctx.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        BadObjectLiteralComparisonMessage::Array,
        BadObjectLiteralComparisonHint::Array,
      );
    }
  }
}

fn is_empty_object(expr: &Expr) -> bool {
  matches!(expr, Expr::Object(obj) if obj.props.is_empty())
}

fn is_empty_array(expr: &Expr) -> bool {
  matches!(expr, Expr::Array(arr) if arr.elems.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/oxc/bad_object_literal_comparison.rs
  // MIT Licensed.

  #[test]
  fn bad_object_literal_comparison_valid() {
    assert_lint_ok! {
      BadObjectLiteralComparison,
      "if (x === null) {}",
      "if (x === undefined) {}",
      "if (typeof x === 'object') {}",
      "if (x === y) {}",
      // Non-empty literals are not flagged.
      "if (x === { a: 1 }) {}",
      "if (x !== [1]) {}",
    };
  }

  #[test]
  fn bad_object_literal_comparison_invalid() {
    assert_lint_err! {
      BadObjectLiteralComparison,
      "if (x === {}) {}": [
        {
          col: 4,
          message: BadObjectLiteralComparisonMessage::Object,
          hint: BadObjectLiteralComparisonHint::Object,
        }
      ],
      "if (x != {}) {}": [
        {
          col: 4,
          message: BadObjectLiteralComparisonMessage::Object,
          hint: BadObjectLiteralComparisonHint::Object,
        }
      ],
      "if (arr !== []) {}": [
        {
          col: 4,
          message: BadObjectLiteralComparisonMessage::Array,
          hint: BadObjectLiteralComparisonHint::Array,
        }
      ],
      // Reversed operand order.
      "[] === x;": [
        {
          col: 0,
          message: BadObjectLiteralComparisonMessage::Array,
          hint: BadObjectLiteralComparisonHint::Array,
        }
      ]
    };
  }
}
