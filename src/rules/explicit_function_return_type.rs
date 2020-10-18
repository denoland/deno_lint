// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct ExplicitFunctionReturnType;

impl LintRule for ExplicitFunctionReturnType {
  fn new() -> Box<Self> {
    Box::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    "explicit-function-return-type"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = ExplicitFunctionReturnTypeVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn docs(&self) -> &'static str {
    r#"Requires all functions to have explicit return types.

Explicit return types have a number of advantages including easier to understand
code and better type safety.  It is clear from the signature what the return 
type of the function (if any) will be.
    
### Valid:
```typescript
function someCalc(): number { return 2*2; }
function anotherCalc(): void { return; }
```

### Invalid:
```typescript
function someCalc() { return 2*2; }
function anotherCalc() { return; }
```"#
  }
}

struct ExplicitFunctionReturnTypeVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ExplicitFunctionReturnTypeVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for ExplicitFunctionReturnTypeVisitor<'c> {
  noop_visit_type!();

  fn visit_function(
    &mut self,
    function: &swc_ecmascript::ast::Function,
    _parent: &dyn Node,
  ) {
    if function.return_type.is_none() {
      self.context.add_diagnostic_with_hint(
        function.span,
        "explicit-function-return-type",
        "Missing return type on function",
        "Add a return type to the function signature",
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
