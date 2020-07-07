// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;

#[allow(unused_imports)]
use swc_ecma_ast::{Expr, ExprOrSuper};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoNonNullAssertedOptionalChain;

impl LintRule for NoNonNullAssertedOptionalChain {
  fn new() -> Box<Self> {
    Box::new(NoNonNullAssertedOptionalChain)
  }

  fn code(&self) -> &'static str {
    "no-non-null-asserted-optional-chain"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoNonNullAssertedOptionalChainVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

#[allow(dead_code)]
struct NoNonNullAssertedOptionalChainVisitor {
  context: Context,
}

impl NoNonNullAssertedOptionalChainVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-non-null-asserted-optional-chain",
      "Optional chain expressions can return undefined by design - using a non-null assertion is unsafe and wrong.",
    );
  }

  #[allow(dead_code)]
  fn check_expr_for_nested_optional_assert(&mut self, span: Span, expr: &Expr) {
    match expr {
      Expr::OptChain(_) => self.add_diagnostic(span),
      // Expr::Call(call_exp) => {
      //   if let ExprOrSuper::Expr(expr) = &call_exp.callee {
      //     self.check_expr_for_nested_optional_assert(span, expr);
      //   }
      // }
      Expr::Paren(paren_expr) => {
        self.check_expr_for_nested_optional_assert(span, &*paren_expr.expr)
      }
      _ => {}
    }
  }
}

impl Visit for NoNonNullAssertedOptionalChainVisitor {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &swc_ecma_ast::TsNonNullExpr,
    _parent: &dyn Node,
  ) {
    // eprintln!("{:#?}", ts_non_null_expr);
    let maybe_expr_or_super = match &*ts_non_null_expr.expr {
      Expr::Member(member_expr) => Some(&member_expr.obj),
      Expr::Call(call_expr) => Some(&call_expr.callee),
      _ => None,
    };

    if let Some(expr_or_super) = maybe_expr_or_super {
      if let ExprOrSuper::Expr(expr) = &expr_or_super {
        self.check_expr_for_nested_optional_assert(ts_non_null_expr.span, expr);
      }
    }

    self.check_expr_for_nested_optional_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
    // swc_ecma_visit::visit_ts_non_null_expr(self, ts_non_null_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_non_null_asserted_optional_chain_ok() {
    // assert_lint_ok::<NoNonNullAssertedOptionalChain>("foo.bar!;");
    // assert_lint_ok::<NoNonNullAssertedOptionalChain>("foo.bar()!;");
    // assert_lint_ok::<NoNonNullAssertedOptionalChain>("foo?.bar();");
    // assert_lint_ok::<NoNonNullAssertedOptionalChain>("foo?.bar;");
    assert_lint_ok::<NoNonNullAssertedOptionalChain>("(foo?.bar).baz!;");
    // assert_lint_ok::<NoNonNullAssertedOptionalChain>("(foo?.bar()).baz!;");
    // 'foo?.bar();',
    // '(foo?.bar).baz!;',
    // '(foo?.bar()).baz!;',
  }

  #[test]
  fn no_non_null_asserted_optional_chain_err() {
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.bar!;", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.['bar']!;", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("foo?.bar()!;", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("foo.bar?.()!;", 0);
    assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!.baz", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!().baz", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar)!()", 0);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar!)", 1);
    // assert_lint_err::<NoNonNullAssertedOptionalChain>("(foo?.bar!)()", 1);
  }
}
