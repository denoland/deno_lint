// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::WithStmt;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoWith;

impl LintRule for NoWith {
  fn new() -> Box<Self> {
    Box::new(NoWith)
  }

  fn code(&self) -> &'static str {
    "no-with"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecmascript::ast::Module) {
    let mut visitor = NoWithVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoWithVisitor {
  context: Arc<Context>,
}

impl NoWithVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoWithVisitor {
  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _parent: &dyn Node) {
    self.context.add_diagnostic(
      with_stmt.span,
      "no-with",
      "`with` statement is not allowed",
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_with() {
    assert_lint_err::<NoWith>("with (someVar) { console.log('asdf'); }", 0)
  }
}
