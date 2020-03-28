use super::Context;
use crate::traverse::AstTraverser;

pub struct ExplicitFunctionReturnType {
  context: Context,
}

impl ExplicitFunctionReturnType {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for ExplicitFunctionReturnType {
  fn walk_function(&self, function: swc_ecma_ast::Function) {
    if function.return_type.is_none() {
      self.context.add_diagnostic(
        &function.span,
        "explicitFunctionReturnType",
        "Missing return type on function",
      );
    }
  }
}
