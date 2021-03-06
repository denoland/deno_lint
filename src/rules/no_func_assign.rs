// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};
use derive_more::Display;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoFuncAssign;

const CODE: &str = "no-func-assign";

#[derive(Display)]
enum NoFuncAssignMessage {
  #[display(fmt = "Reassigning function declaration is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoFuncAssignHint {
  #[display(
    fmt = "Remove or rework the reassignment of the existing function"
  )]
  RemoveOrRework,
}

impl LintRule for NoFuncAssign {
  fn new() -> Box<Self> {
    Box::new(NoFuncAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoFuncAssignVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
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
}

struct NoFuncAssignVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoFuncAssignVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> VisitAll for NoFuncAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ids = find_lhs_ids(&assign_expr.left);

    for id in ids {
      let var = self.context.scope.var(&id);
      if let Some(var) = var {
        if let BindingKind::Function = var.kind() {
          self.context.add_diagnostic_with_hint(
            assign_expr.span,
            CODE,
            NoFuncAssignMessage::Unexpected,
            NoFuncAssignHint::RemoveOrRework,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.13.0/tests/lib/rules/no-func-assign.js
  // MIT Licensed.

  #[test]
  fn no_func_assign_valid() {
    assert_lint_ok! {
      NoFuncAssign,
      "function foo() { var foo = bar; }",
      "function foo(foo) { foo = bar; }",
      "function foo() { var foo; foo = bar; }",
      "var foo = () => {}; foo = bar;",
      "var foo = function() {}; foo = bar;",
      "var foo = function() { foo = bar; };",
      "import bar from 'bar'; function foo() { var foo = bar; }",
    };
  }

  #[test]
  fn no_func_assign_invalid() {
    assert_lint_err! {
      NoFuncAssign,
      "function foo() {}; foo = bar;": [
        {
          col: 19,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { foo = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "foo = bar; function foo() { };": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "[foo] = bar; function foo() { }": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "({x: foo = 0} = bar); function foo() { };": [
        {
          col: 1,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { [foo] = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "(function() { ({x: foo = 0} = bar); function foo() { }; })();": [
        {
          col: 15,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "var a = function foo() { foo = 123; };": [
        {
          col: 25,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      r#"
const a = "a";
const unused = "unused";

function asdf(b: number, c: string): number {
    console.log(a, b);
    debugger;
    return 1;
}

asdf = "foobar";
      "#: [
        {
          col: 0,
          line: 11,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],

      // nested
      r#"
function foo() {}
let a;
a = () => {
  foo = 42;
};
      "#: [
        {
          line: 5,
          col: 2,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
    };
  }
}
