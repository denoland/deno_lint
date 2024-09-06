use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use if_chain::if_chain;

use deno_ast::{
  view::{BinaryOp, NodeTrait},
  SourceRanged,
};
use derive_more::Display;
#[derive(Debug)]
pub struct NoSelfCompare;

const CODE: &str = "no-self-compare";
const HINT: &str =
  "Comparing a value to itself may be redundant and could potentially indicate a mistake in your code. Please double-check your logic";

#[derive(Display)]
enum NoSelfCompareMessage {
  #[display("`{}` is compared to itself", _0)]
  Invalid(String),
}

impl LintRule for NoSelfCompare {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: deno_ast::view::Program<'view>,
  ) {
    NoSelfCompareHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_self_compare.md")
  }
}

struct NoSelfCompareHandler;

impl Handler for NoSelfCompareHandler {
  fn bin_expr(
    &mut self,
    binary_expression: &deno_ast::view::BinExpr,
    ctx: &mut Context,
  ) {
    if_chain! {
      if matches!(
        binary_expression.op(),
        BinaryOp::EqEqEq
          | BinaryOp::EqEq
          | BinaryOp::NotEqEq
          | BinaryOp::NotEq
          | BinaryOp::Gt
          | BinaryOp::Lt
          | BinaryOp::GtEq
          | BinaryOp::LtEq
      );

      if binary_expression.left.text() == binary_expression.right.text();

      then {
        ctx.add_diagnostic_with_hint(
          binary_expression.range(),
          CODE,
          NoSelfCompareMessage::Invalid(binary_expression.left.text().to_string()),
          HINT,
        )
      }

    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_self_compare_valid() {
    assert_lint_ok! {
        NoSelfCompare,
        "if (x === y) { }",
        "if (1 === 2) { }",
        "y=x*x",
        "foo.bar.baz === foo.bar.qux",
        "if ('x' === 'y') { }",
    };
  }

  #[test]
  fn no_self_compare_invalid() {
    assert_lint_err! {
        NoSelfCompare,
        "if (x === x) { }": [
            {
                line: 1,
                col: 4,
                message: variant!(NoSelfCompareMessage, Invalid, "x"),
                hint: HINT,
            }
        ],
        "if (x == x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
      ],
        "if (x !== x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],
        "if (x > x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],
        "if (x < x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],
        "if (x >= x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],
        "if (x <= x) { }": [
          {
              line: 1,
              col: 4,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],
        "foo.bar().baz.qux >= foo.bar().baz.qux": [
          {
              line: 1,
              col: 0,
              message: variant!(NoSelfCompareMessage, Invalid, "foo.bar().baz.qux"),
              hint: HINT,
          }
        ],
        "foo.bar().baz['qux'] >= foo.bar().baz['qux']": [
          {
              line: 1,
              col: 0,
              message: variant!(NoSelfCompareMessage, Invalid, "foo.bar().baz['qux']"),
              hint: HINT,
          }
        ],
        "if ('x' > 'x') { }": [
          {
            line: 1,
            col: 4,
            message: variant!(NoSelfCompareMessage, Invalid, "'x'"),
            hint: HINT,
          }
        ],
        "do {} while (x === x)": [
          {
            line: 1,
            col: 13,
            message: variant!(NoSelfCompareMessage, Invalid, "x"),
            hint: HINT,
          }
        ],
        "x === x ? console.log('foo') : console.log('bar');": [
          {
              line: 1,
              col: 0,
              message: variant!(NoSelfCompareMessage, Invalid, "x"),
              hint: HINT,
          }
        ],

    };
  }
}
