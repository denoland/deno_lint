// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};
use derive_more::Display;

use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

pub struct NoExAssign;

const CODE: &str = "no-ex-assign";

#[derive(Display)]
enum NoExAssignMessage {
  #[display(fmt = "Reassigning exception parameter is not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoExAssignHint {
  #[display(fmt = "Use a different variable for the assignment")]
  UseDifferent,
}

impl LintRule for NoExAssign {
  fn new() -> Box<Self> {
    Box::new(NoExAssign)
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
    let mut visitor = NoExAssignVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
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
}

struct NoExAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoExAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoExAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _: &dyn Node) {
    let ids = find_lhs_ids(&assign_expr.left);

    for id in ids {
      let var = self.context.scope().var(&id);

      if let Some(var) = var {
        if let BindingKind::CatchClause = var.kind() {
          self.context.add_diagnostic_with_hint(
            assign_expr.span,
            CODE,
            NoExAssignMessage::NotAllowed,
            NoExAssignHint::UseDifferent,
          );
        }
      }
    }
  }
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
      r#"try {} catch (e) { e = 1; }"#: [
        {
          col: 19,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { ex = 1; }"#: [
        {
          col: 20,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { [ex] = []; }"#: [
        {
          col: 20,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { ({x: ex = 0} = {}); }"#: [
        {
          col: 21,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch ({message}) { message = 1; }"#: [
        {
          col: 27,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],

      // nested
      r#"a = () => { try {} catch (e) { e = 1; } };"#: [
        {
          col: 31,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
    };
  }
}
