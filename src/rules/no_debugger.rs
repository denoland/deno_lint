// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::DebuggerStmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDebugger {
  context: Context,
}

impl NoDebugger {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDebugger {
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
