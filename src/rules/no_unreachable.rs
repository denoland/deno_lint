// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Spanned;
use swc_ecmascript::ast::Stmt;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

use std::sync::Arc;

pub struct NoUnreachable;

impl LintRule for NoUnreachable {
  fn new() -> Box<Self> {
    Box::new(NoUnreachable)
  }

  fn code(&self) -> &'static str {
    "no-unreachable"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoUnreachableVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoUnreachableVisitor {
  context: Arc<Context>,
}

impl NoUnreachableVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoUnreachableVisitor {
  fn visit_stmt(&mut self, stmt: &Stmt, _: &dyn Node) {
    stmt.visit_children_with(self);

    if let Some(meta) = self.context.control_flow.meta(stmt.span().lo) {
      if meta.unreachable {
        self.context.add_diagnostic(
          stmt.span(),
          "no-unreachable",
          "This statement is unreachable",
        )
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { function bar() { return 1; } return bar(); }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { return bar(); function bar() { return 1; } }",
    );

    assert_lint_ok::<NoUnreachable>("function foo() { return x; var x; }");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoUnreachable>("function foo() { var x = 1; var y = 2; }");

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; var y = 2; return; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: x = 1; x = 2;} }",
    );
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoUnreachable>("while (true) { break; var x; }");

    assert_lint_ok::<NoUnreachable>("while (true) { continue; var x, y; }");

    assert_lint_ok::<NoUnreachable>("while (true) { throw 'message'; var x; }");
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoUnreachable>(
      "while (true) { if (true) break; var x = 1; }",
    );

    assert_lint_ok::<NoUnreachable>("while (true) continue;");

    assert_lint_ok::<NoUnreachable>("switch (foo) { case 1: break; var x; }");
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoUnreachable>(
      "switch (foo) { case 1: break; var x; default: throw true; };",
    );

    assert_lint_ok::<NoUnreachable>("const arrow_direction = arrow => {  switch (arrow) { default: throw new Error();  };}");

    assert_lint_ok::<NoUnreachable>("var x = 1; y = 2; throw 'uh oh'; var y;");
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; if (x) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; if (x) { } else { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>("function foo() { var x = 1; switch (x) { case 0: break; default: return; } x = 2; }");
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; while (x) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; for (x in {}) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; try { return; } finally { x = 2; } }",
    );
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; for (;;) { if (x) break; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>("A: { break A; } foo()");

    assert_lint_ok::<NoUnreachable>("function* foo() { try { yield 1; return; } catch (err) { return err; } }");
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { try { bar(); return; } catch (err) { return err; } }",
    );

    assert_lint_ok::<NoUnreachable>("function foo() { try { a.b.c = 1; return; } catch (err) { return err; } }");
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { return x; var x = 1; }",
      27,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { return x; var x, y = 1; }",
      27,
    );

    assert_lint_err::<NoUnreachable>(
      "while (true) { continue; var x = 1; }",
      25,
    );
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoUnreachable>("function foo() { return; x = 1; }", 25);

    assert_lint_err::<NoUnreachable>(
      "function foo() { throw error; x = 1; }",
      30,
    );

    assert_lint_err::<NoUnreachable>("while (true) { break; x = 1; }", 22);
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoUnreachable>("while (true) { continue; x = 1; }", 25);

    assert_lint_err::<NoUnreachable>(
      "function foo() { switch (foo) { case 1: return; x = 1; } }",
      48,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { switch (foo) { case 1: throw e; x = 1; } }",
      49,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: break; x = 1; } }",
      0,
    );

    assert_lint_err::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: continue; x = 1; } }",
      0,
    );

    assert_lint_err::<NoUnreachable>("var x = 1; throw 'uh oh'; var y = 2;", 0);
  }

  #[test]
  fn err_5() {
    assert_lint_err::<NoUnreachable>("function foo() { var x = 1; if (x) { return; } else { throw e; } x = 2; }", 0);

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; if (x) return; else throw -1; x = 2; }",
      0,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; try { return; } finally {} x = 2; }",
      0,
    );
  }

  #[test]
  fn err_6() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; try { } finally { return; } x = 2; }",
      0,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; do { return; } while (x); x = 2; }",
      0,
    );

    assert_lint_err::<NoUnreachable>("function foo() { var x = 1; while (x) { if (x) break; else continue; x = 2; } }", 0);
  }

  #[test]
  fn err_7() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; for (;;) { if (x) continue; } x = 2; }",
      0,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; while (true) { } x = 2; }",
      0,
    );

    assert_lint_err::<NoUnreachable>("const arrow_direction = arrow => {  switch (arrow) { default: throw new Error();  }; g() }", 0);
  }

  // TODO: Copy https://github.com/eslint/eslint/blob/4111d21a046b73892e2c84f92815a21ef4db63e1/tests/lib/rules/no-unreachable.js#L106
}
