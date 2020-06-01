// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecma_ast::{BlockStmt, Module, SwitchStmt};
use swc_ecma_visit::{Node, Visit};

pub struct NoEmpty;

impl LintRule for NoEmpty {
  fn new() -> Box<Self> {
    Box::new(NoEmpty)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoEmptyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoEmptyVisitor {
  context: Context,
}

impl NoEmptyVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyVisitor {
  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _parent: &dyn Node) {
    if block_stmt.stmts.is_empty() {
      if !block_stmt.contains_comments(&self.context) {
        self.context.add_diagnostic(
          block_stmt.span,
          "noEmpty",
          "Empty block statement",
        );
      }
    } else {
      for stmt in &block_stmt.stmts {
        self.visit_stmt(stmt, _parent);
      }
    }
  }

  fn visit_switch_stmt(&mut self, switch: &SwitchStmt, _parent: &dyn Node) {
    if switch.cases.is_empty() {
      self.context.add_diagnostic(
        switch.span,
        "noEmpty",
        "Empty switch statement",
      );
    }
  }
}

trait ContainsComments {
  fn contains_comments(&self, context: &Context) -> bool;
}

impl ContainsComments for BlockStmt {
  fn contains_comments(&self, context: &Context) -> bool {
    context
      .leading_comments
      .iter()
      .flat_map(|r| r.value().clone())
      .any(|comment| self.span.contains(comment.span))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_for_a_non_empty_block() {
    assert_lint_ok::<NoEmpty>(r#"if (foo) { var bar = ""; }"#);
  }

  #[test]
  fn it_passes_for_a_block_only_containing_comments() {
    assert_lint_ok::<NoEmpty>(
      r#"
if (foo) {
  // This block is not empty
}
    "#,
    );
  }

  #[test]
  fn it_passes_for_a_non_empty_switch_block() {
    assert_lint_ok::<NoEmpty>(
      r#"
    switch (foo) {
      case bar:
        break;
    }
      "#,
    );
  }

  #[test]
  fn it_passes_for_a_non_empty_nested_block() {
    assert_lint_ok::<NoEmpty>(
      r#"
if (foo) {
  if (bar) {
    var baz = "";
  }
}
      "#,
    );
  }

  #[test]
  fn it_fails_for_an_empty_if_block() {
    assert_lint_err::<NoEmpty>("if (foo) { }", "noEmpty", 9);
  }

  #[test]
  fn it_fails_for_an_empty_block_with_preceding_comments() {
    assert_lint_err_on_line::<NoEmpty>(
      r#"
// This is an empty block
if (foo) { }
      "#,
      "noEmpty",
      3,
      9,
    );
  }

  #[test]
  fn it_fails_for_an_empty_while_block() {
    assert_lint_err::<NoEmpty>("while (foo) { }", "noEmpty", 12);
  }

  #[test]
  fn it_fails_for_an_empty_do_while_block() {
    assert_lint_err::<NoEmpty>("do { } while (foo);", "noEmpty", 3);
  }

  #[test]
  fn it_fails_for_an_empty_for_block() {
    assert_lint_err::<NoEmpty>("for(;;) { }", "noEmpty", 8);
  }

  #[test]
  fn it_fails_for_an_empty_for_in_block() {
    assert_lint_err::<NoEmpty>("for(var foo in bar) { }", "noEmpty", 20);
  }

  #[test]
  fn it_fails_for_an_empty_for_of_block() {
    assert_lint_err::<NoEmpty>("for(var foo of bar) { }", "noEmpty", 20);
  }

  #[test]
  fn it_fails_for_an_empty_switch_block() {
    assert_lint_err::<NoEmpty>("switch (foo) { }", "noEmpty", 0);
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_block() {
    assert_lint_err_n::<NoEmpty>(
      "try { } catch (err) { }",
      vec![("noEmpty", 4), ("noEmpty", 20)],
    );
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_finally_block() {
    assert_lint_err_n::<NoEmpty>(
      "try { } catch (err) { } finally { }",
      vec![("noEmpty", 4), ("noEmpty", 20), ("noEmpty", 32)],
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_if_block() {
    assert_lint_err::<NoEmpty>("if (foo) { if (bar) { } }", "noEmpty", 20);
  }

  #[test]
  fn it_fails_for_a_nested_empty_while_block() {
    assert_lint_err::<NoEmpty>("if (foo) { while (bar) { } }", "noEmpty", 23);
  }

  #[test]
  fn it_fails_for_a_nested_empty_do_while_block() {
    assert_lint_err::<NoEmpty>(
      "if (foo) { do { } while (bar); }",
      "noEmpty",
      14,
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(;;) { } }", "noEmpty", 19);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_in_block() {
    assert_lint_err::<NoEmpty>(
      "if (foo) { for(var bar in foo) { } }",
      "noEmpty",
      31,
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_of_block() {
    assert_lint_err::<NoEmpty>(
      "if (foo) { for(var bar of foo) { } }",
      "noEmpty",
      31,
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_switch() {
    assert_lint_err::<NoEmpty>("if (foo) { switch (foo) { } }", "noEmpty", 11);
  }
}
