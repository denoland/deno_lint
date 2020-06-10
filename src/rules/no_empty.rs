// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecma_ast::{BlockStmt, Function, Module, SwitchStmt};
use swc_ecma_visit::{Node, Visit};

pub struct NoEmpty;

impl LintRule for NoEmpty {
  fn new() -> Box<Self> {
    Box::new(NoEmpty)
  }

  fn code(&self) -> &'static str {
    "no-empty"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoEmptyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoEmptyVisitor {
  context: Context,
}

impl NoEmptyVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyVisitor {
  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    // Empty functions shouldn't be caught be this rule.
    // Because function's body is a block statement, we're gonna
    // manually visit each member; otherwise rule would produce errors
    // for empty function body.
    if let Some(body) = &function.body {
      for stmt in &body.stmts {
        swc_ecma_visit::visit_stmt(self, stmt, body);
      }
    }
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _parent: &dyn Node) {
    if block_stmt.stmts.is_empty() {
      if !block_stmt.contains_comments(&self.context) {
        self.context.add_diagnostic(
          block_stmt.span,
          "no-empty",
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
        "no-empty",
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
  fn it_passes_for_a_function() {
    assert_lint_ok::<NoEmpty>(r#"function foobar() {}"#);
  }

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
    assert_lint_err::<NoEmpty>("if (foo) { }", 9);
  }

  #[test]
  fn it_fails_for_an_empty_block_with_preceding_comments() {
    assert_lint_err_on_line::<NoEmpty>(
      r#"
// This is an empty block
if (foo) { }
      "#,
      3,
      9,
    );
  }

  #[test]
  fn it_fails_for_an_empty_while_block() {
    assert_lint_err::<NoEmpty>("while (foo) { }", 12);
  }

  #[test]
  fn it_fails_for_an_empty_do_while_block() {
    assert_lint_err::<NoEmpty>("do { } while (foo);", 3);
  }

  #[test]
  fn it_fails_for_an_empty_for_block() {
    assert_lint_err::<NoEmpty>("for(;;) { }", 8);
  }

  #[test]
  fn it_fails_for_an_empty_for_in_block() {
    assert_lint_err::<NoEmpty>("for(var foo in bar) { }", 20);
  }

  #[test]
  fn it_fails_for_an_empty_for_of_block() {
    assert_lint_err::<NoEmpty>("for(var foo of bar) { }", 20);
  }

  #[test]
  fn it_fails_for_an_empty_switch_block() {
    assert_lint_err::<NoEmpty>("switch (foo) { }", 0);
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_block() {
    assert_lint_err_n::<NoEmpty>("try { } catch (err) { }", vec![4, 20]);
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_finally_block() {
    assert_lint_err_n::<NoEmpty>(
      "try { } catch (err) { } finally { }",
      vec![4, 20, 32],
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_if_block() {
    assert_lint_err::<NoEmpty>("if (foo) { if (bar) { } }", 20);
  }

  #[test]
  fn it_fails_for_a_nested_empty_while_block() {
    assert_lint_err::<NoEmpty>("if (foo) { while (bar) { } }", 23);
  }

  #[test]
  fn it_fails_for_a_nested_empty_do_while_block() {
    assert_lint_err::<NoEmpty>("if (foo) { do { } while (bar); }", 14);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(;;) { } }", 19);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_in_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(var bar in foo) { } }", 31);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_of_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(var bar of foo) { } }", 31);
  }

  #[test]
  fn it_fails_for_a_nested_empty_switch() {
    assert_lint_err::<NoEmpty>("if (foo) { switch (foo) { } }", 11);
  }
}
