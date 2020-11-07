// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::scoped_rule::ScopeRule;
use crate::scoped_rule::ScopedRule;
use crate::scopes::BindingKind;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::utils::ident::IdentLike;

pub type NoExAssign = ScopedRule<NoExAssignImpl>;
pub struct NoExAssignImpl;

const CODE: &str = "no-ex-assign";
const MESSAGE: &str = "Reassigning exception parameter is not allowed";
const HINT: &str = "Use a different variable for the assignment";

impl ScopeRule for NoExAssignImpl {
  fn new() -> Self {
    NoExAssignImpl
  }

  fn tags() -> &'static [&'static str] {
    &["recommended"]
  }

  fn code() -> &'static str {
    CODE
  }

  fn docs() -> &'static str {
    r#"Disallows the reassignment of exception parameters

There is generally no good reason to reassign an exception parameter.  Once
reassigned the code from that point on has no reference to the error anymore.

### Invalid:
```typescript
try {
  someFunc();
} catch (e) {
  e = true;
  // can no longer access the thrown error
}
```

### Valid:
```typescript
try {
  someFunc();
} catch (e) {
  const anotherVar = true;
}
```
"#
  }

  fn check_assignment(&mut self, context: &mut Context, i: &Ident) {
    let var = context.scope.var(&i.to_id());

    if let Some(var) = var {
      if let BindingKind::CatchClause = var.kind() {
        context.add_diagnostic_with_hint(i.span, CODE, MESSAGE, HINT);
      }
    }
  }

  fn check_usage(&mut self, _: &mut Context, _: &Ident) {}
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_ex_assign_valid() {
    assert_lint_ok! {
      NoExAssign,
      r#"
try {} catch { e = 1; }
try {} catch (ex) { something = 1; }
try {} catch (ex) { return 1; }
function foo() { try { } catch (e) { return false; } }
      "#,
    };
  }

  #[test]
  fn no_ex_assign_invalid() {
    assert_lint_err! {
      NoExAssign,
      r#"
try {} catch (e) { e = 1; }
try {} catch (ex) { ex = 1; }
try {} catch (ex) { [ex] = []; }
try {} catch (ex) { ({x: ex = 0} = {}); }
try {} catch ({message}) { message = 1; }
      "#: [
        {
          line: 2,
          col: 19,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 3,
          col: 20,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 4,
          col: 21,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 5,
          col: 25,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 6,
          col: 27,
          message: MESSAGE,
          hint: HINT,
        },
      ]
    }
  }
}
