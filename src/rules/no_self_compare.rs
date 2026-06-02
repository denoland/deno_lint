use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;
use derive_more::Display;

#[derive(Debug)]
pub struct NoSelfCompare;

const CODE: &str = "no-self-compare";
const HINT: &str =
  "Comparing a value to itself may be redundant and could potentially indicate a mistake in your code. Please double-check your logic";

#[derive(Display)]
enum NoSelfCompareMessage {
  #[display(fmt = "`{}` is compared to itself", _0)]
  Invalid(String),
}

impl LintRule for NoSelfCompare {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoSelfCompareHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoSelfCompareHandler;

impl Handler<'_> for NoSelfCompareHandler {
  fn binary_expression(
    &mut self,
    binary_expression: &BinaryExpression,
    ctx: &mut Context,
  ) {
    if !matches!(
      binary_expression.operator,
      BinaryOperator::StrictEquality
        | BinaryOperator::Equality
        | BinaryOperator::StrictInequality
        | BinaryOperator::Inequality
        | BinaryOperator::GreaterThan
        | BinaryOperator::LessThan
        | BinaryOperator::GreaterEqualThan
        | BinaryOperator::LessEqualThan
    ) {
      return;
    }

    let src = ctx.source_text();
    let left_span = binary_expression.left.span();
    let right_span = binary_expression.right.span();
    let left_text = &src[left_span.start as usize..left_span.end as usize];
    let right_text = &src[right_span.start as usize..right_span.end as usize];

    if left_text == right_text {
      ctx.add_diagnostic_with_hint(
        binary_expression.span,
        CODE,
        NoSelfCompareMessage::Invalid(left_text.to_string()),
        HINT,
      )
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
