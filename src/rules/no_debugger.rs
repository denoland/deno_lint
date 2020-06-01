// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::DebuggerStmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDebugger;

impl LintRule for NoDebugger {
  fn new() -> Box<Self> {
    Box::new(NoDebugger)
  }

  fn code(&self) -> &'static str {
    "noDebugger"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoDebuggerVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}
pub struct NoDebuggerVisitor {
  context: Context,
}

impl NoDebuggerVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDebuggerVisitor {
  fn visit_debugger_stmt(
    &mut self,
    debugger_stmt: &DebuggerStmt,
    _parent: &dyn Node,
  ) {
    self.context.add_diagnostic(
      debugger_stmt.span,
      "noDebugger",
      "`debugger` statement is not allowed",
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_debugger_test() {
    assert_lint_err::<NoDebugger>(
      r#"function asdf(): number { console.log("asdf"); debugger; return 1; }"#,
      47,
    )
  }
}
