use super::Context;
use swc_common::Visit;
use swc_common::VisitWith;

pub struct NoVar {
  context: Context,
}

impl NoVar {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl<T> Visit<T> for NoVar
where
  T: VisitWith<Self> + std::fmt::Debug,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::VarDecl> for NoVar {
  fn visit(&mut self, node: &swc_ecma_ast::VarDecl) {
    if node.kind == swc_ecma_ast::VarDeclKind::Var {
      self.context.add_diagnostic(
        &node.span,
        "noVar",
        "`var` keyword is not allowed",
      );
    }
  }
}
