// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::scoped_rule::ScopeRule;
use crate::scoped_rule::ScopedRule;
use crate::scopes::BindingKind;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::utils::ident::IdentLike;

pub type NoFuncAssign = ScopedRule<NoFuncAssignImpl>;

pub struct NoFuncAssignImpl;

impl ScopeRule for NoFuncAssignImpl {
  fn new() -> Self {
    NoFuncAssignImpl
  }

  fn tags() -> &'static [&'static str] {
    &["recommended"]
  }

  fn code() -> &'static str {
    "no-func-assign"
  }

  fn docs() -> &'static str {
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
const someVar = foo;

const a = function baz() {
  const someStr = "now I'm a string";
}

const anotherFuncRef = existingFunc;

let myFuncVar = function() {}
myFuncVar = bar;  // variable reassignment, not function re-declaration
```
"#
  }

  fn check_assignment(&mut self, context: &mut Context, i: &Ident) {
    let var = context.scope.var(&i.to_id());
    if let Some(var) = var {
      if let BindingKind::Function = var.kind() {
        context.add_diagnostic_with_hint(
          i.span,
          "no-func-assign",
          "Reassigning function declaration is not allowed",
          "Remove or rework the reassignment of the existing function",
        );
      }
    }
  }

  fn check_usage(&mut self, _: &mut Context, _: &Ident) {}
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
