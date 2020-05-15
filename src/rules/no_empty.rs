use super::{LintRule, Context};
use swc_ecma_ast::{Module, BlockStmt};
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
    println!("{:?}", block_stmt.stmts);
    if block_stmt.stmts.is_empty() {
      self.context.add_diagnostic(
        block_stmt.span,
        "noEmpty",
        "Empty block statement",
      );
    }
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
      json!([])
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
      json!([])
    )
  }

  #[test]
  fn it_fails_for_an_empty_block() {
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
      }])
    )
  }
}