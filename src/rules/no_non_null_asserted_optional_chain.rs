// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Expr, ExprOrSuper};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use swc_common::Span;

pub struct NoNonNullAssertedOptionalChain;

impl LintRule for NoNonNullAssertedOptionalChain {
  fn new() -> Box<Self> {
    Box::new(NoNonNullAssertedOptionalChain)
  }

  fn code(&self) -> &'static str {
    "no-non-null-asserted-optional-chain"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoNonNullAssertedOptionalChainVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoNonNullAssertedOptionalChainVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoNonNullAssertedOptionalChainVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-non-null-asserted-optional-chain",
      "Optional chain expressions can return undefined by design - using a non-null assertion is unsafe and wrong.",
    );
  }

  fn check_expr_for_nested_optional_assert(&mut self, span: Span, expr: &Expr) {
    if let Expr::OptChain(_) = expr {
      self.add_diagnostic(span)
    }
  }
}

impl<'c> Visit for NoNonNullAssertedOptionalChainVisitor<'c> {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &swc_ecmascript::ast::TsNonNullExpr,
    _parent: &dyn Node,
  ) {
    match &*ts_non_null_expr.expr {
      Expr::Member(member_expr) => {
        if let ExprOrSuper::Expr(expr) = &member_expr.obj {
          self
            .check_expr_for_nested_optional_assert(ts_non_null_expr.span, expr);
        }
      }
      Expr::Call(call_expr) => {
        if let ExprOrSuper::Expr(expr) = &call_expr.callee {
          self
            .check_expr_for_nested_optional_assert(ts_non_null_expr.span, expr);
        }
      }
      Expr::Paren(paren_expr) => self.check_expr_for_nested_optional_assert(
        ts_non_null_expr.span,
        &*paren_expr.expr,
      ),
      _ => {}
    };

    self.check_expr_for_nested_optional_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_non_null_asserted_optional_chain_valid() {
    assert_lint_ok! {
      NoNonNullAssertedOptionalChain,
      "foo.bar!;",
      "foo.bar()!;",
      "foo?.bar();",
      "foo?.bar;",
      "(foo?.bar).baz!;",
      "(foo?.bar()).baz!;",
    };
  }

  #[test]
  fn no_non_null_asserted_optional_chain_invalid() {
    assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.bar!;", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.['bar']!;", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.bar()!;", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("foo.bar?.()!;", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!.baz", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!().baz", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!()", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar!)", 1);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar!)()", 1);
  }
}
