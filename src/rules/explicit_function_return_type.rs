// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct ExplicitFunctionReturnType {
  context: Context,
}

impl ExplicitFunctionReturnType {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for ExplicitFunctionReturnType {
  fn visit_function(
    &mut self,
    function: &swc_ecma_ast::Function,
    _parent: &dyn Node,
  ) {
    if function.return_type.is_none() {
      self.context.add_diagnostic(
        function.span,
        "explicitFunctionReturnType",
        "Missing return type on function",
      );
    }
  }
}
