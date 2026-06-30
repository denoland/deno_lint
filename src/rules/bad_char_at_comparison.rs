// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::BinaryOp;
use deno_ast::view::{BinExpr, Callee, Expr, Lit, MemberProp};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct BadCharAtComparison;

const CODE: &str = "bad-char-at-comparison";

#[derive(Display)]
enum BadCharAtComparisonMessage {
  #[display(
    fmt = "`charAt` returns a single character, so comparing its result against a string of a different length is always false"
  )]
  Unexpected,
}

#[derive(Display)]
enum BadCharAtComparisonHint {
  #[display(
    fmt = "Compare against a single character, or use `startsWith`/`slice` to compare longer substrings"
  )]
  Fix,
}

impl LintRule for BadCharAtComparison {
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
    BadCharAtComparisonHandler.traverse(program, context);
  }
}

struct BadCharAtComparisonHandler;

impl Handler for BadCharAtComparisonHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, ctx: &mut Context) {
    if !matches!(
      bin_expr.op(),
      BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
    ) {
      return;
    }

    let bad = (is_char_at_call(&bin_expr.left)
      && is_bad_comparison_string(&bin_expr.right))
      || (is_char_at_call(&bin_expr.right)
        && is_bad_comparison_string(&bin_expr.left));

    if bad {
      ctx.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        BadCharAtComparisonMessage::Unexpected,
        BadCharAtComparisonHint::Fix,
      );
    }
  }
}

fn is_char_at_call(expr: &Expr) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };
  let Callee::Expr(callee) = call.callee else {
    return false;
  };
  let Expr::Member(member) = callee else {
    return false;
  };
  let MemberProp::Ident(prop) = member.prop else {
    return false;
  };
  prop.sym() == "charAt" && call.args.len() == 1
}

fn is_bad_comparison_string(expr: &Expr) -> bool {
  let Expr::Lit(Lit::Str(s)) = expr else {
    return false;
  };
  let value = s.value().to_string_lossy();
  // `charAt` yields exactly one UTF-16 code unit. An empty string or a single
  // character may legitimately be compared; anything longer can never match.
  value.len() >= 2 && value.chars().count() != 1
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/oxc/bad_char_at_comparison.rs
  // MIT Licensed.

  #[test]
  fn bad_char_at_comparison_valid() {
    assert_lint_ok! {
      BadCharAtComparison,
      "a.charAt(4) === 'a';",
      "a.charAt(4) === '\\n';",
      "a.charAt(4) !== 'b';",
      // Not a `charAt` call.
      "chatAt(4) === 'a2';",
      "a.foo(4) === 'a2';",
      // Right-hand side is not a string literal.
      "a.charAt(4) === 'a' + 'b';",
      "a.charAt(4) === x;",
    };
  }

  #[test]
  fn bad_char_at_comparison_invalid() {
    assert_lint_err! {
      BadCharAtComparison,
      "a.charAt(4) === 'aa';": [
        {
          col: 0,
          message: BadCharAtComparisonMessage::Unexpected,
          hint: BadCharAtComparisonHint::Fix,
        }
      ],
      "a.charAt(822) !== 'foo';": [
        {
          col: 0,
          message: BadCharAtComparisonMessage::Unexpected,
          hint: BadCharAtComparisonHint::Fix,
        }
      ],
      // Reversed operand order.
      "'aa' === a.charAt(4);": [
        {
          col: 0,
          message: BadCharAtComparisonMessage::Unexpected,
          hint: BadCharAtComparisonHint::Fix,
        }
      ]
    };
  }
}
