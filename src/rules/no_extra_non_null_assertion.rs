// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::OptChainExpr;
use swc_ecmascript::ast::TsNonNullExpr;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoExtraNonNullAssertion;

impl LintRule for NoExtraNonNullAssertion {
  fn new() -> Box<Self> {
    Box::new(NoExtraNonNullAssertion)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-extra-non-null-assertion"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoExtraNonNullAssertionVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows unnecessary non-null assertions

Non-null assertions are specified with an `!` saying to the compiler that you 
know this value is not null.  Specifying this operator more than once in a row,
or in combination with the optional chaining operator (`?`) is confusing and
unnecessary.

### Invalid:
```typescript
const foo: { str: string } | null = null; 
const bar = foo!!.str;

function myFunc(bar: undefined | string) { return bar!!; }
function anotherFunc(bar?: { str: string }) { return bar!?.str; }
```

### Valid:
```typescript
const foo: { str: string } | null = null; 
const bar = foo!.str;

function myFunc(bar: undefined | string) { return bar!; }
function anotherFunc(bar?: { str: string }) { return bar?.str; }
```
"#
  }
}

struct NoExtraNonNullAssertionVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoExtraNonNullAssertionVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      "no-extra-non-null-assertion",
      "Extra non-null assertion is forbidden",
      "Remove the extra non-null assertion operator (`!`)",
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

impl<'c> Visit for NoExtraNonNullAssertionVisitor<'c> {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &TsNonNullExpr,
    parent: &dyn Node,
  ) {
    self.check_expr_for_nested_non_null_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
    swc_ecmascript::visit::visit_ts_non_null_expr(
      self,
      ts_non_null_expr,
      parent,
    );
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

    swc_ecmascript::visit::visit_opt_chain_expr(self, opt_chain_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
