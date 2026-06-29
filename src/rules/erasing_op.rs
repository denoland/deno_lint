// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::swc::ast::BinaryOp;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::Lit;
use deno_ast::swc::ast::ParenExpr;
use deno_ast::view::BinExpr;
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct ErasingOp;

const CODE: &str = "erasing-op";

#[derive(Display)]
enum ErasingOpMessage {
  #[display(fmt = "This operation will always evaluate to zero")]
  Unexpected,
}

#[derive(Display)]
enum ErasingOpHint {
  #[display(
    fmt = "This is most likely a mistake; remove the operation or assign zero directly"
  )]
  Remove,
}

impl LintRule for ErasingOp {
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
    ErasingOpHandler.traverse(program, context);
  }
}

struct ErasingOpHandler;

impl Handler for ErasingOpHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, context: &mut Context) {
    let inner = bin_expr.inner;

    let erases = match inner.op {
      // `x * 0`, `0 * x`, `x & 0`, `0 & x` all evaluate to zero.
      BinaryOp::Mul | BinaryOp::BitAnd => {
        is_zero(&inner.left) || is_zero(&inner.right)
      }
      // `0 / x` is zero, but `0 / 0` is `NaN`, so don't flag a zero divisor.
      BinaryOp::Div => is_zero(&inner.left) && !is_zero(&inner.right),
      _ => false,
    };

    if erases {
      context.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        ErasingOpMessage::Unexpected,
        ErasingOpHint::Remove,
      );
    }
  }
}

fn is_zero(expr: &Expr) -> bool {
  match expr {
    Expr::Paren(ParenExpr { expr, .. }) => is_zero(expr),
    Expr::Lit(Lit::Num(num)) => num.value == 0.0,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/oxc/erasing_op.rs
  // which is in turn based on the Clippy `erasing_op` lint. MIT Licensed.

  #[test]
  fn erasing_op_valid() {
    assert_lint_ok! {
      ErasingOp,
      "x * 1;",
      "1 * x;",
      "5 & x;",
      "x / 1;",
      "1 / x;",
      // `0 / 0` is `NaN`, not zero.
      "0 / 0;",
      // `x / 0` is `Infinity` / `NaN`, not zero.
      "x / 0;",
      // Operators that are not erasing.
      "x + 0;",
      "x - 0;",
      "x | 0;",
      "0 - x;",
    };
  }

  #[test]
  fn erasing_op_invalid() {
    assert_lint_err! {
      ErasingOp,
      "x * 0;": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ],
      "0 * x;": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ],
      "x & 0;": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ],
      "0 & x;": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ],
      "0 / x;": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ],
      // Parenthesized zero is still zero.
      "x * (0);": [
        {
          col: 0,
          message: ErasingOpMessage::Unexpected,
          hint: ErasingOpHint::Remove,
        }
      ]
    };
  }
}
