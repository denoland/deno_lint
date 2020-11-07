// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::scoped_rule::ScopeRule;
use crate::scoped_rule::ScopedRule;
use crate::scopes::BindingKind;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::utils::ident::IdentLike;

pub type NoClassAssign = ScopedRule<NoClassAssignImpl>;

pub struct NoClassAssignImpl;

const CODE: &str = "no-class-assign";
const MESSAGE: &str = "Reassigning class declaration is not allowed";
const HINT: &str = "Do you have the right variable here?";

impl ScopeRule for NoClassAssignImpl {
  fn new() -> Self {
    NoClassAssignImpl
  }

  fn tags() -> &'static [&'static str] {
    &["recommended"]
  }

  fn code() -> &'static str {
    CODE
  }

  fn docs() -> &'static str {
    r#"Disallows modifying variables of class declarations

Declaring a class such as `class A{}`, creates a variable `A`.  Like any variable
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

  fn check_assignment(&mut self, context: &mut Context, i: &Ident) {
    let var = context.scope.var(&i.to_id());
    if let Some(var) = var {
      if let BindingKind::Class = var.kind() {
        context.add_diagnostic_with_hint(i.span, CODE, MESSAGE, HINT);
      }
    }
  }

  fn check_usage(&mut self, _: &mut Context, _: &Ident) {}
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
          col: 2,
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
          col: 5,
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
