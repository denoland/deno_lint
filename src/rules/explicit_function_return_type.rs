// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct ExplicitFunctionReturnType;

impl LintRule for ExplicitFunctionReturnType {
  fn new() -> Box<Self> {
    Box::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    "explicit-function-return-type"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = ExplicitFunctionReturnTypeVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct ExplicitFunctionReturnTypeVisitor {
  context: Arc<Context>,
}

impl ExplicitFunctionReturnTypeVisitor {
  pub fn new(context: Arc<Context>) -> Self {
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
        "explicit-function-return-type",
        "Missing return type on function",
      );
    }
    for stmt in &function.body {
      self.visit_block_stmt(stmt, _parent);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn explicit_function_return_type_valid() {
    assert_lint_ok_n::<ExplicitFunctionReturnType>(vec![
      "function fooTyped(): void { }",
      "const bar = (a: string) => { }",
      "const barTyped = (a: string): Promise<void> => { }",
    ]);
  }

  #[test]
  fn explicit_function_return_type_invalid() {
    assert_lint_err::<ExplicitFunctionReturnType>("function foo() { }", 0);
    assert_lint_err_on_line_n::<ExplicitFunctionReturnType>(
      r#"
function a() {
  function b() {}
}
      "#,
      vec![(2, 0), (3, 2)],
    );
  }
}
