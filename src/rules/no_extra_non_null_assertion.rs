// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprOrSuper;
use swc_ecma_ast::OptChainExpr;
use swc_ecma_ast::TsNonNullExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoExtraNonNullAssertion;

impl LintRule for NoExtraNonNullAssertion {
  fn new() -> Box<Self> {
    Box::new(NoExtraNonNullAssertion)
  }

  fn code(&self) -> &'static str {
    "no-extra-non-null-assertion"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoExtraNonNullAssertionVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoExtraNonNullAssertionVisitor {
  context: Context,
}

impl NoExtraNonNullAssertionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-extra-non-null-assertion",
      "Extra non-null assertion is forbidden",
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

impl Visit for NoExtraNonNullAssertionVisitor {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &TsNonNullExpr,
    parent: &dyn Node,
  ) {
    self.check_expr_for_nested_non_null_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
    swc_ecma_visit::visit_ts_non_null_expr(self, ts_non_null_expr, parent);
  }

  fn visit_opt_chain_expr(
    &mut self,
    opt_chain_expr: &OptChainExpr,
    parent: &dyn Node,
  ) {
    let maybe_expr_or_super = match &*opt_chain_expr.expr {
      Expr::Member(member_expr) => Some(&member_expr.obj),
      Expr::Call(call_expr) => Some(&call_expr.callee),
      _ => None,
    };

    if let Some(expr_or_super) = maybe_expr_or_super {
      if let ExprOrSuper::Expr(expr) = &expr_or_super {
        self.check_expr_for_nested_non_null_assert(opt_chain_expr.span, expr);
      }
    }

    swc_ecma_visit::visit_opt_chain_expr(self, opt_chain_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_extra_non_null_assertion_ok() {
    assert_lint_ok::<NoExtraNonNullAssertion>(
      r#"const foo: { str: string } | null = null; const bar = foo!.str;"#,
    );
    assert_lint_ok::<NoExtraNonNullAssertion>(
      r#"function foo() { return "foo"; }"#,
    );
    assert_lint_ok::<NoExtraNonNullAssertion>(
      r#"function foo(bar: undefined | string) { return bar!; }"#,
    );
    assert_lint_ok::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return bar?.str; }"#,
    );
  }

  #[test]
  fn no_extra_non_null_assertion_err() {
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"const foo: { str: string } | null = null; const bar = foo!!.str;"#,
      54,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar: undefined | string) { return bar!!; }"#,
      47,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return bar!?.str; }"#,
      45,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return (bar!)!.str; }"#,
      45,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return (bar!)?.str; }"#,
      45,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return bar!?.(); }"#,
      45,
    );
    assert_lint_err::<NoExtraNonNullAssertion>(
      r#"function foo(bar?: { str: string }) { return (bar!)?.(); }"#,
      45,
    );
  }
}
