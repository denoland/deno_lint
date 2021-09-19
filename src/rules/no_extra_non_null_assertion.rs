// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ExprOrSuper;
use deno_ast::swc::ast::OptChainExpr;
use deno_ast::swc::ast::TsNonNullExpr;
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::{VisitAll, VisitAllWith};
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

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoExtraNonNullAssertionVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_extra_non_null_assertion.md")
  }
}

struct NoExtraNonNullAssertionVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoExtraNonNullAssertionVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoExtraNonNullAssertionMessage::Unexpected,
      NoExtraNonNullAssertionHint::Remove,
    );
  }

  fn check_expr_for_nested_non_null_assert(&mut self, span: Span, expr: &Expr) {
    match expr {
      Expr::TsNonNull(_) => self.add_diagnostic(span),
      Expr::Paren(paren_expr) => {
        self.check_expr_for_nested_non_null_assert(span, &*paren_expr.expr)
      }
      _ => {}
    }
  }
}

impl<'c, 'view> VisitAll for NoExtraNonNullAssertionVisitor<'c, 'view> {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &TsNonNullExpr,
    _: &dyn Node,
  ) {
    self.check_expr_for_nested_non_null_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
  }

  fn visit_opt_chain_expr(
    &mut self,
    opt_chain_expr: &OptChainExpr,
    _: &dyn Node,
  ) {
    let maybe_expr_or_super = match &*opt_chain_expr.expr {
      Expr::Member(member_expr) => Some(&member_expr.obj),
      Expr::Call(call_expr) => Some(&call_expr.callee),
      _ => None,
    };

    if let Some(ExprOrSuper::Expr(expr)) = maybe_expr_or_super {
      self.check_expr_for_nested_non_null_assert(opt_chain_expr.span, expr);
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
