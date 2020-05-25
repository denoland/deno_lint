// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecma_ast::{BlockStmt, Module, Stmt, SwitchStmt};
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

  fn get_nested_block_stmt(&self, stmt: &Stmt) -> Option<BlockStmt> {
    if let Some(Stmt::Block(block)) = match stmt {
      Stmt::If(if_stmt) => Some(if_stmt.cons.as_ref()),
      Stmt::While(while_stmt) => Some(while_stmt.body.as_ref()),
      Stmt::DoWhile(do_while) => Some(do_while.body.as_ref()),
      Stmt::For(for_stmt) => Some(for_stmt.body.as_ref()),
      Stmt::ForIn(for_in) => Some(for_in.body.as_ref()),
      Stmt::ForOf(for_of) => Some(for_of.body.as_ref()),
      _ => None,
    } {
      return Some(block.clone());
    } else if let Some(block) = match stmt {
      Stmt::Try(try_stmt) => Some(try_stmt.block),
      _ => None,
    } {
      return block;
    }

    None
  }
}

impl Visit for NoEmptyVisitor {
  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _parent: &dyn Node) {
    if block_stmt.stmts.is_empty() {
      println!("Block is empty");
      if !block_stmt.contains_comments(&self.context) {
        self.context.add_diagnostic(
          block_stmt.span,
          "noEmpty",
          "Empty block statement",
        );
      }
    } else {
      for stmt in &block_stmt.stmts {
        if let Some(block) = self.get_nested_block_stmt(stmt) {
          self.visit_block_stmt(&block, _parent);
        }

        if let Stmt::Switch(switch) = stmt {
          self.visit_switch_stmt(switch, _parent);
        }
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
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn it_passes_for_a_non_empty_block() {
    test_lint(
      "no_empty",
      r#"
if (foo) {
  var bar = "";
}
      "#,
      vec![NoEmpty::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_for_a_block_only_containing_comments() {
    test_lint(
      "no_empty",
      r#"
if (foo) {
  // This block is not empty
}
      "#,
      vec![NoEmpty::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_for_a_non_empty_switch_block() {
    test_lint(
      "no_empty",
      r#"
switch (foo) {
  case bar:
    break;
}
      "#,
      vec![NoEmpty::new()],
      json!([]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_if_block() {
    test_lint(
      "no_empty",
      r#"
if (foo) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 9,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_block_with_preceding_comments() {
    test_lint(
      "no_empty",
      r#"
// This is an empty block
if (foo) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 3,
          "col": 9,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_while_block() {
    test_lint(
      "no_empty",
      r#"
while (foo) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 12,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_do_while_block() {
    test_lint(
      "no_empty",
      r#"
do {
} while (foo);
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 3,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_for_block() {
    test_lint(
      "no_empty",
      r#"
for(;;) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 8,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_for_in_block() {
    test_lint(
      "no_empty",
      r#"
for(var foo in bar) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 20,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_for_of_block() {
    test_lint(
      "no_empty",
      r#"
for(var foo of bar) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 20,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_switch_block() {
    test_lint(
      "no_empty",
      r#"
switch (foo) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty switch statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 0,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_block() {
    test_lint(
      "no_empty",
      r#"
try {
} catch (err) {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 4,
        }
      },
      {
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 3,
          "col": 14,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_finally_block() {
    test_lint(
      "no_empty",
      r#"
try {
} catch (err) {
} finally {
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 2,
          "col": 4,
        }
      },
      {
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 3,
          "col": 14,
        }
      },
      {
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 4,
          "col": 10,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_a_nested_empty_if_block() {
    test_lint(
      "no_empty",
      r#"
if (foo) {
  if (bar) {
  }
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty block statement",
        "location": {
          "filename": "no_empty",
          "line": 3,
          "col": 11,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_a_nested_empty_switch() {
    test_lint(
      "no_empty",
      r#"
if (foo) {
  switch (foo) {
  }
}
      "#,
      vec![NoEmpty::new()],
      json!([{
        "code": "noEmpty",
        "message": "Empty switch statement",
        "location": {
          "filename": "no_empty",
          "line": 3,
          "col": 2,
        }
      }]),
    )
  }
}
