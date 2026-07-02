// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::tags;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{
  ChainElement, ChainExpression, Expression, Program, TSNonNullExpression,
};
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct NoExtraNonNullAssertion;

const CODE: &str = "no-extra-non-null-assertion";

#[derive(Display)]
enum NoExtraNonNullAssertionMessage {
  #[display(fmt = "Extra non-null assertion is forbidden")]
  Unexpected,
}

#[derive(Display)]
enum NoExtraNonNullAssertionHint {
  #[display(fmt = "Remove the extra non-null assertion operator (`!`)")]
  Remove,
}

impl LintRule for NoExtraNonNullAssertion {
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
    let mut handler = NoExtraNonNullAssertionHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoExtraNonNullAssertionHandler;

fn add_diagnostic(span: Span, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoExtraNonNullAssertionMessage::Unexpected,
    NoExtraNonNullAssertionHint::Remove,
  );
}

fn check_expr_for_nested_non_null_assert(
  span: Span,
  expr: &Expression,
  ctx: &mut Context,
) {
  match expr {
    Expression::TSNonNullExpression(_) => add_diagnostic(span, ctx),
    Expression::ParenthesizedExpression(paren_expr) => {
      check_expr_for_nested_non_null_assert(span, &paren_expr.expression, ctx)
    }
    _ => {}
  }
}

impl Handler<'_> for NoExtraNonNullAssertionHandler {
  fn ts_non_null_expression(
    &mut self,
    ts_non_null_expr: &TSNonNullExpression,
    ctx: &mut Context,
  ) {
    check_expr_for_nested_non_null_assert(
      ts_non_null_expr.span,
      &ts_non_null_expr.expression,
      ctx,
    );
  }

  fn chain_expression(
    &mut self,
    chain_expr: &ChainExpression,
    ctx: &mut Context,
  ) {
    let (optional, expr) = match &chain_expr.expression {
      ChainElement::CallExpression(call_expr) => {
        (call_expr.optional, &call_expr.callee)
      }
      ChainElement::StaticMemberExpression(member_expr) => {
        (member_expr.optional, &member_expr.object)
      }
      ChainElement::ComputedMemberExpression(member_expr) => {
        (member_expr.optional, &member_expr.object)
      }
      ChainElement::PrivateFieldExpression(member_expr) => {
        (member_expr.optional, &member_expr.object)
      }
      ChainElement::TSNonNullExpression(_) => return,
    };

    if optional {
      check_expr_for_nested_non_null_assert(chain_expr.span, expr, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_extra_non_null_assertion_valid() {
    assert_lint_ok! {
      NoExtraNonNullAssertion,
      r#"const foo: { str: string } | null = null; const bar = foo!.str;"#,
      r#"function foo() { return "foo"; }"#,
      r#"function foo(bar: undefined | string) { return bar!; }"#,
      r#"function foo(bar?: { str: string }) { return bar?.str; }"#,
      r#"function foo(bar?: { str: string }) { return bar?.str!.length; }"#,
      r#"function foo(bar?: { arr?: number[] }) { return bar?.arr![0]; }"#,
    };
  }

  #[test]
  fn no_extra_non_null_assertion_invalid() {
    assert_lint_err! {
      NoExtraNonNullAssertion,
      r#"const foo: { str: string } | null = null; const bar = foo!!.str;"#: [
        {
          col: 54,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar: undefined | string) { return bar!!; }"#: [
        {
          col: 47,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return bar!?.str; }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return (bar!)!.str; }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return (bar!)?.str; }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return bar!?.(); }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return (bar!)?.(); }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ],
      r#"function foo(bar?: { str: string }) { return bar?.str!?.length; }"#: [
        {
          col: 45,
          message: NoExtraNonNullAssertionMessage::Unexpected,
          hint: NoExtraNonNullAssertionHint::Remove,
        }
      ]
    };
  }
}
