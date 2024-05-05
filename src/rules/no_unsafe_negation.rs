// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::{view as ast_view, SourceRanged};
use derive_more::Display;
use if_chain::if_chain;

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
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoUnsafeNegationHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unsafe_negation.md")
  }
}

struct NoUnsafeNegationHandler;

impl Handler for NoUnsafeNegationHandler {
  fn bin_expr(&mut self, bin_expr: &ast_view::BinExpr, ctx: &mut Context) {
    use deno_ast::view::{BinaryOp, Expr, UnaryOp};
    if_chain! {
      if matches!(bin_expr.op(), BinaryOp::In | BinaryOp::InstanceOf);
      if let Expr::Unary(unary_expr) = &bin_expr.left;
      if unary_expr.op() == UnaryOp::Bang;
      then {
        ctx.add_diagnostic_with_hint(
          bin_expr.range(),
          CODE,
          NoUnsafeNegationMessage::Unexpected(bin_expr.op().to_string()),
          HINT,
        );
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
