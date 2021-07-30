// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct NoClassAssign;

const CODE: &str = "no-class-assign";
const MESSAGE: &str = "Reassigning class declaration is not allowed";
const HINT: &str = "Do you have the right variable here?";

impl LintRule for NoClassAssign {
  fn new() -> Box<Self> {
    Box::new(NoClassAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoClassAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows modifying variables of class declarations

Declaring a class such as `class A {}`, creates a variable `A`.  Like any variable
this can be modified or reassigned. In most cases this is a mistake and not what
was intended.

### Invalid:
```typescript
class A {}
A = 0;  // reassigning the class variable itself
```

### Valid:
```typescript
class A{}
let c = new A();
c = 0;  // reassigning the variable `c`
```
"#
  }
}

struct NoClassAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoClassAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoClassAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ids = find_lhs_ids(&assign_expr.left);
    for id in ids {
      let var = self.context.scope().var(&id);
      if let Some(var) = var {
        if let BindingKind::Class = var.kind() {
          self.context.add_diagnostic_with_hint(
            assign_expr.span,
            CODE,
            MESSAGE,
            HINT,
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
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-class-assign.js
  // MIT Licensed.

  #[test]
  fn no_class_assign_valid() {
    assert_lint_ok! {
      NoClassAssign,
      r#"class A {}"#,
      r#"class A {} foo(A);"#,
      r#"let A = class A {}; foo(A);"#,
      r#"
class A {
  foo(A) {
    A = "foobar";
  }
}
"#,
      r#"
class A {
  foo() {
    let A;
    A = "bar";
  }
}
"#,
      r#"
let A = class {
  b() {
    A = 0;
  }
}
"#,
      r#"
let A, B;
A = class {
  b() {
    B = 0;
  }
}
"#,
      r#"let x = 0; x = 1;"#,
      r#"var x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"function x() {} x = 1;"#,
      r#"function foo(x) { x = 1; }"#,
      r#"try {} catch (x) { x = 1; }"#,
    };
  }

  #[test]
  fn no_class_assign_invalid() {
    assert_lint_err! {
      NoClassAssign,
      r#"
class A {}
A = 0;
      "#: [
        {
          line: 3,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {}
({A} = 0);
      "#: [
        {
          line: 3,
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {}
({b: A = 0} = {});
      "#: [
        {
          line: 3,
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
A = 0;
class A {}
      "#: [
        {
          line: 2,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {
  foo() {
    A = 0;
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
let A = class A {
  foo() {
    A = 0;
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {}
A = 10;
A = 20;
      "#: [
        {
          line: 3,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 4,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
let A;
A = class {
  foo() {
    class B {}
    B = 0;
  }
}
      "#: [
        {
          line: 6,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
