// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::Program;

use deno_ast::view::Expr;
use deno_ast::view::OptChainBase;
use deno_ast::view::OptChainExpr;
use deno_ast::view::TsNonNullExpr;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
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
    NoExtraNonNullAssertionHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_extra_non_null_assertion.md")
  }
}

struct NoExtraNonNullAssertionHandler;

fn add_diagnostic(range: SourceRange, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    range,
    CODE,
    NoExtraNonNullAssertionMessage::Unexpected,
    NoExtraNonNullAssertionHint::Remove,
  );
}

fn check_expr_for_nested_non_null_assert(
  range: SourceRange,
  expr: &Expr,
  ctx: &mut Context,
) {
  match expr {
    Expr::TsNonNull(_) => add_diagnostic(range, ctx),
    Expr::Paren(paren_expr) => {
      check_expr_for_nested_non_null_assert(range, &paren_expr.expr, ctx)
    }
    _ => {}
  }
}

impl Handler for NoExtraNonNullAssertionHandler {
  fn ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &TsNonNullExpr,
    ctx: &mut Context,
  ) {
    check_expr_for_nested_non_null_assert(
      ts_non_null_expr.range(),
      &ts_non_null_expr.expr,
      ctx,
    );
  }

  fn opt_chain_expr(
    &mut self,
    opt_chain_expr: &OptChainExpr,
    ctx: &mut Context,
  ) {
    let expr = match &opt_chain_expr.base {
      OptChainBase::Member(member_expr) => &member_expr.obj,
      OptChainBase::Call(call_expr) => &call_expr.callee,
    };
    check_expr_for_nested_non_null_assert(opt_chain_expr.range(), expr, ctx);
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
      ]
    };
  }
}
