use super::Context;
use swc_common::Visit;
use swc_common::VisitWith;

pub struct SingleVarDeclarator {
  context: Context,
}

impl SingleVarDeclarator {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl<T> Visit<T> for SingleVarDeclarator
where
  T: VisitWith<Self> + std::fmt::Debug,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::VarDecl> for SingleVarDeclarator {
  fn visit(&mut self, node: &swc_ecma_ast::VarDecl) {
    if node.decls.len() > 1 {
      self.context.add_diagnostic(
        &node.span,
        "singleVarDeclarator",
        "Multiple variable declarators are not allowed",
      );
    }
  }
}
