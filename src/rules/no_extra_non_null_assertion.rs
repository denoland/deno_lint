// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{Expr, ExprOrSuper, OptChainExpr, TsNonNullExpr};
use derive_more::Display;
use std::sync::Arc;

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
  fn new() -> Arc<Self> {
    Arc::new(NoExtraNonNullAssertion)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
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
  expr: &Expr,
  ctx: &mut Context,
) {
  match expr {
    Expr::TsNonNull(_) => add_diagnostic(span, ctx),
    Expr::Paren(paren_expr) => {
      check_expr_for_nested_non_null_assert(span, &paren_expr.expr, ctx)
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
      ts_non_null_expr.span(),
      &ts_non_null_expr.expr,
      ctx,
    );
  }

  fn opt_chain_expr(
    &mut self,
    opt_chain_expr: &OptChainExpr,
    ctx: &mut Context,
  ) {
    let maybe_expr_or_super = match opt_chain_expr.expr {
      Expr::Member(member_expr) => Some(&member_expr.obj),
      Expr::Call(call_expr) => Some(&call_expr.callee),
      _ => None,
    };

    if let Some(ExprOrSuper::Expr(expr)) = maybe_expr_or_super {
      check_expr_for_nested_non_null_assert(opt_chain_expr.span(), expr, ctx);
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
