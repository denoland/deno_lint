// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct ExplicitFunctionReturnType;

impl LintRule for ExplicitFunctionReturnType {
  fn new() -> Box<Self> {
    Box::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    "explicitFunctionReturnType"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = ExplicitFunctionReturnTypeVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct ExplicitFunctionReturnTypeVisitor {
  context: Context,
}

impl ExplicitFunctionReturnTypeVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for ExplicitFunctionReturnTypeVisitor {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn explicit_function_return_type() {
    assert_lint_ok_n::<ExplicitFunctionReturnType>(vec![
      "function fooTyped(): void { }",
      "const bar = (a: string) => { }",
      "const barTyped = (a: string): Promise<void> => { }",
    ]);
    assert_lint_err::<ExplicitFunctionReturnType>("function foo() { }", 0);
  }
}
