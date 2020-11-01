// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoFuncAssign;

impl LintRule for NoFuncAssign {
  fn new() -> Box<Self> {
    Box::new(NoFuncAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-func-assign"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoFuncAssignVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the overwriting/reassignment of an existing function

Javascript allows for the reassignment of a function definition.  This is
generally a mistake on the developers part, or poor coding practice as code
readability and maintainability will suffer.
    
### Invalid:
```typescript
function foo() {}
foo = bar;

const a = function baz() {
  baz = "now I'm a string";
}

myFunc = existingFunc;
function myFunc() {}
```

### Valid:
```typescript
function foo() {}
const someVar = bar;

const a = function baz() {
  const someStr = "now I'm a string";
}

myFunc = existingFunc;
function myFunc() {}

const myFuncVar = function() {}
myFuncVar = bar;  // variable reassignment, not function re-declaration
```
"#
  }
}

struct NoFuncAssignVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoFuncAssignVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoFuncAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ids = find_lhs_ids(&assign_expr.left);

    for id in ids {
      let var = self.context.scope.var(&id);
      if let Some(var) = var {
        if let BindingKind::Function = var.kind() {
          self.context.add_diagnostic_with_hint(
            assign_expr.span,
            "no-func-assign",
            "Reassigning function declaration is not allowed",
            "Remove or rework the reassignment of the existing function",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line;

  #[test]
  fn no_func_assign() {
    assert_lint_err_on_line::<NoFuncAssign>(
      r#"
const a = "a";
const unused = "unused";

function asdf(b: number, c: string): number {
    console.log(a, b);
    debugger;
    return 1;
}

asdf = "foobar";
      "#,
      11,
      0,
    );
  }
}
