use super::Context;
use swc_common::Visit;
use swc_common::VisitWith;

pub struct NoDebugger {
  context: Context,
}

impl NoDebugger {
  pub fn new(context: Context) -> Self {
    Self { context }
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
