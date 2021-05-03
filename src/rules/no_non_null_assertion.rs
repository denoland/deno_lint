// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
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

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoNonNullAssertionVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct NoNonNullAssertionVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoNonNullAssertionVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoNonNullAssertionVisitor<'c, 'view> {
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
