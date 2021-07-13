// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};

use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

const BANNED_PROPERTIES: &[&str] =
  &["hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable"];

pub struct NoPrototypeBuiltins;

const CODE: &str = "no-prototype-builtins";

fn get_message(prop: &str) -> String {
  format!(
    "Access to Object.prototype.{} is not allowed from target object",
    prop
  )
}

impl LintRule for NoPrototypeBuiltins {
  fn new() -> Box<Self> {
    Box::new(NoPrototypeBuiltins)
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
    let mut visitor = NoPrototypeBuiltinsVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of `Object.prototype` builtins directly

If objects are created via `Object.create(null)` they have no prototype
specified. This can lead to runtime errors when you assume objects have
properties from `Object.prototype` and attempt to call the following methods:

- `hasOwnProperty`
- `isPrototypeOf`
- `propertyIsEnumerable`

Instead, it's always encouraged to call these methods from `Object.prototype`
explicitly.

### Invalid:

```typescript
const a = foo.hasOwnProperty("bar");
const b = foo.isPrototypeOf("bar");
const c = foo.propertyIsEnumerable("bar");
```

### Valid:

```typescript
const a = Object.prototype.hasOwnProperty.call(foo, "bar");
const b = Object.prototype.isPrototypeOf.call(foo, "bar");
const c = Object.prototype.propertyIsEnumerable.call(foo, "bar");
```
"#
  }
}

struct NoPrototypeBuiltinsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoPrototypeBuiltinsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoPrototypeBuiltinsVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    let member_expr = match &call_expr.callee {
      ExprOrSuper::Expr(boxed_expr) => match &**boxed_expr {
        Expr::Member(member_expr) => {
          if member_expr.computed {
            return;
          }
          member_expr
        }
        _ => return,
      },
      ExprOrSuper::Super(_) => return,
    };

    if let Expr::Ident(ident) = &*member_expr.prop {
      let prop_name = ident.sym.as_ref();
      if BANNED_PROPERTIES.contains(&prop_name) {
        self.context.add_diagnostic(
          call_expr.span,
          CODE,
          get_message(prop_name),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_prototype_builtins_valid() {
    assert_lint_ok! {
      NoPrototypeBuiltins,
      r#"
  Object.prototype.hasOwnProperty.call(foo, "bar");
  Object.prototype.isPrototypeOf.call(foo, "bar");
  Object.prototype.propertyIsEnumerable.call(foo, "bar");
  Object.prototype.hasOwnProperty.apply(foo, ["bar"]);
  Object.prototype.isPrototypeOf.apply(foo, ["bar"]);
  Object.prototype.propertyIsEnumerable.apply(foo, ["bar"]);
  hasOwnProperty(foo, "bar");
  isPrototypeOf(foo, "bar");
  propertyIsEnumerable(foo, "bar");
  ({}.hasOwnProperty.call(foo, "bar"));
  ({}.isPrototypeOf.call(foo, "bar"));
  ({}.propertyIsEnumerable.call(foo, "bar"));
  ({}.hasOwnProperty.apply(foo, ["bar"]));
  ({}.isPrototypeOf.apply(foo, ["bar"]));
  ({}.propertyIsEnumerable.apply(foo, ["bar"]));
      "#,
    };
  }

  #[test]
  fn no_prototype_builtins_invalid() {
    assert_lint_err! {
      NoPrototypeBuiltins,
      "foo.hasOwnProperty('bar');": [{col: 0, message: get_message("hasOwnProperty")}],
      "foo.isPrototypeOf('bar');": [{col: 0, message: get_message("isPrototypeOf")}],
      "foo.propertyIsEnumerable('bar');": [{col: 0, message: get_message("propertyIsEnumerable")}],
      "foo.bar.baz.hasOwnProperty('bar');": [{col: 0, message: get_message("hasOwnProperty")}],
    }
  }
}
