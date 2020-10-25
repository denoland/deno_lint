// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoNonNullAssertion;

impl LintRule for NoNonNullAssertion {
  fn new() -> Box<Self> {
    Box::new(NoNonNullAssertion)
  }

  fn code(&self) -> &'static str {
    "no-non-null-assertion"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoNonNullAssertionVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoNonNullAssertionVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoNonNullAssertionVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoNonNullAssertionVisitor<'c> {
  fn visit_ts_non_null_expr(
    &mut self,
    non_null_expr: &swc_ecmascript::ast::TsNonNullExpr,
    _parent: &dyn Node,
  ) {
    self.context.add_diagnostic(
      non_null_expr.span,
      "no-non-null-assertion",
      "do not use non-null assertion",
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_non_null_assertion_valid() {
    assert_lint_ok! {
      NoNonNullAssertion,
      "instance.doWork();",
      "foo.bar?.includes('baz')",
      "x;",
      "x.y;",
      "x.y.z;",
      "x?.y.z;",
      "x?.y?.z;",
      "!x;",
    };
  }

  #[test]
  fn no_non_null_assertion_invalid() {
    assert_lint_err::<NoNonNullAssertion>("instance!.doWork()", 0);
    assert_lint_err::<NoNonNullAssertion>("foo.bar!.includes('baz');", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y.z!?.();", 0);
    assert_lint_err::<NoNonNullAssertion>("x!?.y.z;", 0);
    assert_lint_err::<NoNonNullAssertion>("x!?.[y].z;", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y.z!!();", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y!!;", 0);
    assert_lint_err::<NoNonNullAssertion>("x!!.y;", 0);
    assert_lint_err::<NoNonNullAssertion>("x!!!;", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y?.z!();", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y.z!();", 0);
    assert_lint_err::<NoNonNullAssertion>("x![y]?.z;", 0);
    assert_lint_err::<NoNonNullAssertion>("x![y];", 0);
    assert_lint_err::<NoNonNullAssertion>("!x!.y;", 1);
    assert_lint_err::<NoNonNullAssertion>("x!.y?.z;", 0);
    assert_lint_err::<NoNonNullAssertion>("x.y!;", 0);
    assert_lint_err::<NoNonNullAssertion>("x!.y;", 0);
    assert_lint_err::<NoNonNullAssertion>("x!;", 0);
  }
}
