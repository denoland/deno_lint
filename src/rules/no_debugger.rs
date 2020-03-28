use super::Context;
use crate::traverse::AstTraverser;
use swc_common::Visit;
use swc_common::VisitWith;
use swc_ecma_ast::DebuggerStmt;

pub struct NoDebugger {
  context: Context,
}

impl NoDebugger {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoDebugger {
  fn walk_debugger_stmt(&self, debugger_stmt: DebuggerStmt) {
    self.context.add_diagnostic(
      &debugger_stmt.span,
      "noDebugger",
      "`debugger` statement is not allowed",
    );
  }
}

impl<T> Visit<T> for NoDebugger
where
  T: VisitWith<Self>,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::DebuggerStmt> for NoDebugger {
  fn visit(&mut self, node: &swc_ecma_ast::DebuggerStmt) {
    self.context.add_diagnostic(
      &node.span,
      "noDebugger",
      "`debugger` statement is not allowed",
    );
  }
}
