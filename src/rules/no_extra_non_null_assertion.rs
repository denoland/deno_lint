// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsNonNullExpr;
use swc_ecma_ast::ExprOrSuper;
use swc_ecma_ast::OptChainExpr;
use swc_ecma_ast::Expr;
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
    eprintln!("module {:#?}", module);
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
}

impl Visit for NoExtraNonNullAssertionVisitor {
  fn visit_ts_non_null_expr(&mut self, ts_non_null_expr: &TsNonNullExpr, parent: &dyn Node) {
    if let Expr::TsNonNull(_non_null_expr) = &*ts_non_null_expr.expr {
      self.context.add_diagnostic(
        ts_non_null_expr.span,
        "no-extra-non-null-assertion",
        "Extra non-null assertion is forbidden",
      );
    }
    swc_ecma_visit::visit_ts_non_null_expr(self, ts_non_null_expr, parent);
  }

  fn visit_opt_chain_expr(&mut self, opt_chain_expr: &OptChainExpr, parent: &dyn Node) {
    eprintln!("opt chain expr {:#?}", opt_chain_expr);

    if let Expr::Member(member_expr) = &*opt_chain_expr.expr {
      if let ExprOrSuper::Expr(expr) = &member_expr.obj {
        if let Expr::TsNonNull(_non_null_expr) = &**expr {
          self.context.add_diagnostic(
            opt_chain_expr.span,
            "no-extra-non-null-assertion",
            "Extra non-null assertion is forbidden",
          );
        }
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
    assert_lint_ok::<NoExtraNonNullAssertion>(r#"const foo: { str: string } | null = null; const bar = foo!.str;"#);
    assert_lint_ok::<NoExtraNonNullAssertion>(r#"function foo() { return "foo"; }"#);
    assert_lint_ok::<NoExtraNonNullAssertion>(r#"function foo(bar: undefined | string) { return bar!; }"#);
    assert_lint_ok::<NoExtraNonNullAssertion>(r#"function foo(bar?: { str: string }) { return bar?.str; }"#);
  }

  #[test]
  fn no_extra_non_null_assertion_err() {
    // assert_lint_err::<NoExtraNonNullAssertion>(r#"const foo: { str: string } | null = null; const bar = foo!!.str;"#, 54);
    // assert_lint_err::<NoExtraNonNullAssertion>(r#"function foo(bar: undefined | string) { return bar!!; }"#, 47);
    assert_lint_err::<NoExtraNonNullAssertion>(r#"function foo(bar?: { str: string }) { return bar!?.str; }"#, 45);
    assert_lint_err::<NoExtraNonNullAssertion>(r#"function foo(bar?: { str: string }) { return (bar!)!.str; }"#, 45);
    // assert_lint_err::<NoExtraNonNullAssertion>(r#"function foo(bar?: { str: string }) { return (bar!)?.str; }"#, 45);
  }
}
