// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_common::Span;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::NewExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoObjCalls;

const CODE: &str = "no-obj-calls";

fn get_message(callee_name: &str) -> String {
  format!("`{}` call as function is not allowed", callee_name)
}

impl LintRule for NoObjCalls {
  fn new() -> Box<Self> {
    Box::new(NoObjCalls)
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
    let mut visitor = NoObjCallsVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows calling built-in global objects like functions

The following built-in objects should not be invoked like functions, even though
they look like constructors:

- `Math`
- `JSON`
- `Reflect`
- `Atomics`

Calling these as functions would result in runtime errors. This rule statically
prevents such wrong usage of them.

### Invalid:

```typescript
const math = Math();
const newMath = new Math();

const json = JSON();
const newJSON = new JSON();

const reflect = Reflect();
const newReflect = new Reflect();

const atomics = Atomics();
const newAtomics = new Atomics();
```

### Valid:

```typescript
const area = (radius: number): number => Math.PI * radius * radius;

const parsed = JSON.parse("{ foo: 42 }");

const x = Reflect.get({ x: 1, y: 2 }, "x");

const first = Atomics.load(foo, 0);
```
"#
  }
}

struct NoObjCallsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoObjCallsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_callee(&mut self, callee_name: impl AsRef<str>, span: Span) {
    let callee_name = callee_name.as_ref();
    match callee_name {
      "Math" | "JSON" | "Reflect" | "Atomics" => {
        self.context.add_diagnostic(
          span,
          "no-obj-calls",
          get_message(callee_name),
        );
      }
      _ => {}
    }
  }
}

impl<'c, 'view> Visit for NoObjCallsVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        self.check_callee(&ident.sym, call_expr.span);
      }
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      self.check_callee(&ident.sym, new_expr.span);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_obj_calls_valid() {
    assert_lint_ok! {
      NoObjCalls,
      "Math.PI * 2 * 3;",
      "JSON.parse(\"{}\");",
      "Reflect.get({ x: 1, y: 2 }, \"x\");",
      "Atomics.load(foo, 0);",
    };
  }

  #[test]
  fn no_obj_calls_invalid() {
    assert_lint_err! {
      NoObjCalls,
      "Math();": [{col: 0, message: get_message("Math")}],
      "new Math();": [{col: 0, message: get_message("Math")}],
      "JSON();": [{col: 0, message: get_message("JSON")}],
      "new JSON();": [{col: 0, message: get_message("JSON")}],
      "Reflect();": [{col: 0, message: get_message("Reflect")}],
      "new Reflect();": [{col: 0, message: get_message("Reflect")}],
      "Atomics();": [{col: 0, message: get_message("Atomics")}],
      "new Atomics();": [{col: 0, message: get_message("Atomics")}],
    }
  }
}
