// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::WithStmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoWith;

impl LintRule for NoWith {
  fn new() -> Box<Self> {
    Box::new(NoWith)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoWithVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoWithVisitor {
  context: Context,
}

impl NoWithVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoWithVisitor {
  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _parent: &dyn Node) {
    self.context.add_diagnostic(
      with_stmt.span,
      "noWith",
      "`with` statement is not allowed",
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn no_with() {
    test_lint(
      "no_with",
      r#"
with (someVar) {
  console.log("asdf");
}
      "#,
      vec![NoWith::new()],
      json!([{
        "code": "noWith",
        "message": "`with` statement is not allowed",
        "location": {
          "filename": "no_with",
          "line": 2,
          "col": 0,
        }
      }]),
    )
  }
}
