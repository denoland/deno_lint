use super::Context;
use swc_common::Span;
use swc_common::Visit;
use swc_common::VisitWith;

pub struct ExplicitFunctionReturnType {
  context: Context,
}

impl ExplicitFunctionReturnType {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl ExplicitFunctionReturnType {
  fn handle(&self, span: &Span) {
    self.context.add_diagnostic(
      span,
      "explicitFunctionReturnType",
      "Missing return type on function",
    );
  }
}

impl<T> Visit<T> for ExplicitFunctionReturnType
where
  T: VisitWith<Self> + std::fmt::Debug,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::FnExpr> for ExplicitFunctionReturnType {
  fn visit(&mut self, node: &swc_ecma_ast::FnExpr) {
    if node.function.return_type.is_none() {
      self.handle(&node.function.span);
    }
  }
}

impl Visit<swc_ecma_ast::FnDecl> for ExplicitFunctionReturnType {
  fn visit(&mut self, node: &swc_ecma_ast::FnDecl) {
    if node.function.return_type.is_none() {
      self.handle(&node.function.span);
    }
  }
}

impl Visit<swc_ecma_ast::ArrowExpr> for ExplicitFunctionReturnType {
  fn visit(&mut self, node: &swc_ecma_ast::ArrowExpr) {
    if node.return_type.is_none() {
      self.handle(&node.span);
    }
  }
}
