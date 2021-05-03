// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::swc_util::StringRepr;
use derive_more::Display;
use std::collections::BTreeMap;
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmtOrExpr, CallExpr, ClassMethod, Expr, ExprOrSuper, FnDecl,
  FnExpr, GetterProp, MethodKind, PrivateMethod, Prop, PropName, PropOrSpread,
  ReturnStmt,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct GetterReturn;

const CODE: &str = "getter-return";

#[derive(Display)]
enum GetterReturnMessage {
  #[display(fmt = "Expected to return a value in '{}'.", _0)]
  Expected(String),
  #[display(fmt = "Expected '{}' to always return a value.", _0)]
  ExpectedAlways(String),
}

#[derive(Display)]
enum GetterReturnHint {
  #[display(fmt = "Return a value from the getter function")]
  Return,
}

impl LintRule for GetterReturn {
  fn new() -> Box<Self> {
    Box::new(GetterReturn)
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
    let mut visitor = GetterReturnVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
    visitor.report();
  }

  fn docs(&self) -> &'static str {
    r#"Requires all property getter functions to return a value

Getter functions return the value of a property.  If the function returns no
value then this contract is broken.

### Invalid:
```typescript
let foo = { 
  get bar() {}
};

class Person { 
  get name() {}
}
```
    
### Valid:
```typescript
let foo = { 
  get bar() { 
    return true; 
  }
};

class Person { 
  get name() { 
    return "alice"; 
  }
}
```
"#
  }
}

struct GetterReturnVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  errors: BTreeMap<Span, GetterReturnMessage>,
  /// If this visitor is currently in a getter, its name is stored.
  getter_name: Option<String>,
  // `true` if a getter contains as least one return statement.
  has_return: bool,
}

impl<'c, 'view> GetterReturnVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self {
      context,
      errors: BTreeMap::new(),
      getter_name: None,
      has_return: false,
    }
  }

  fn report(&mut self) {
    for (span, msg) in &self.errors {
      self.context.add_diagnostic_with_hint(
        *span,
        CODE,
        msg,
        GetterReturnHint::Return,
      );
    }
  }

  fn report_expected(&mut self, span: Span) {
    self.errors.insert(
      span,
      GetterReturnMessage::Expected(
        self
          .getter_name
          .clone()
          .expect("the name of getter is not set"),
      ),
    );
  }

  fn report_always_expected(&mut self, span: Span) {
    self.errors.insert(
      span,
      GetterReturnMessage::ExpectedAlways(
        self
          .getter_name
          .clone()
          .expect("the name of getter is not set"),
      ),
    );
  }

  fn check_getter(&mut self, getter_body_span: Span, getter_span: Span) {
    if self.getter_name.is_none() {
      return;
    }

    if self
      .context
      .control_flow()
      .meta(getter_body_span.lo)
      .unwrap()
      .continues_execution()
    {
      if self.has_return {
        self.report_always_expected(getter_span);
      } else {
        self.report_expected(getter_span);
      }
    }
  }

  fn set_getter_name<S: StringRepr>(&mut self, name: &S) {
    self.getter_name =
      Some(name.string_repr().unwrap_or_else(|| "get".to_string()));
  }

  fn set_default_getter_name(&mut self) {
    self.getter_name = Some("get".to_string());
  }

  fn visit_getter_or_function<F>(&mut self, op: F)
  where
    F: FnOnce(&mut Self),
  {
    let prev_name = self.getter_name.take();
    let prev_has_return = self.has_return;
    op(self);
    self.getter_name = prev_name;
    self.has_return = prev_has_return;
  }
}

impl<'c, 'view> Visit for GetterReturnVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
    // `self.has_return` should be reset because return statements inside the `fn_decl` don't have
    // effect on outside of it
    self.visit_getter_or_function(|a| {
      fn_decl.visit_children_with(a);
    });
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, _: &dyn Node) {
    // `self.has_return` should be reset because return statements inside the `fn_expr` don't have
    // effect on outside of it
    self.visit_getter_or_function(|a| {
      fn_expr.visit_children_with(a);
    });
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    // `self.has_return` should be reset because return statements inside the `arrow_expr` don't
    // have effect on outside of it
    self.visit_getter_or_function(|a| {
      arrow_expr.visit_children_with(a);
    });
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod, _: &dyn Node) {
    self.visit_getter_or_function(|a| {
      if class_method.kind == MethodKind::Getter {
        a.set_getter_name(&class_method.key);
      }
      class_method.visit_children_with(a);

      if let Some(body) = &class_method.function.body {
        a.check_getter(body.span, class_method.span);
      }
    });
  }

  fn visit_private_method(
    &mut self,
    private_method: &PrivateMethod,
    _: &dyn Node,
  ) {
    self.visit_getter_or_function(|a| {
      if private_method.kind == MethodKind::Getter {
        a.set_getter_name(&private_method.key);
      }
      private_method.visit_children_with(a);

      if let Some(body) = &private_method.function.body {
        a.check_getter(body.span, private_method.span);
      }
    });
  }

  fn visit_getter_prop(&mut self, getter_prop: &GetterProp, _: &dyn Node) {
    self.visit_getter_or_function(|a| {
      a.set_getter_name(&getter_prop.key);
      getter_prop.visit_children_with(a);

      if let Some(body) = &getter_prop.body {
        a.check_getter(body.span, getter_prop.span);
      }
    });
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    call_expr.visit_children_with(self);

    if call_expr.args.len() != 3 {
      return;
    }
    if let ExprOrSuper::Expr(callee_expr) = &call_expr.callee {
      if let Expr::Member(member) = &**callee_expr {
        if let ExprOrSuper::Expr(member_obj) = &member.obj {
          if let Expr::Ident(ident) = &**member_obj {
            if ident.sym != *"Object" {
              return;
            }
          }
        }
        if let Expr::Ident(ident) = &*member.prop {
          if ident.sym != *"defineProperty" {
            return;
          }
        }
      }
    }
    if let Expr::Object(obj_expr) = &*call_expr.args[2].expr {
      for prop in obj_expr.props.iter() {
        if let PropOrSpread::Prop(prop_expr) = prop {
          if let Prop::KeyValue(kv_prop) = &**prop_expr {
            // e.g. Object.defineProperty(foo, 'bar', { get: function() {} })
            if let PropName::Ident(ident) = &kv_prop.key {
              if ident.sym != *"get" {
                return;
              }

              self.visit_getter_or_function(|a| {
                if let Expr::Fn(fn_expr) = &*kv_prop.value {
                  a.set_getter_name(&fn_expr.ident);
                  if let Some(body) = &fn_expr.function.body {
                    body.visit_children_with(a);
                    a.check_getter(body.span, prop.span());
                  }
                } else if let Expr::Arrow(arrow_expr) = &*kv_prop.value {
                  a.set_default_getter_name();
                  if let BlockStmtOrExpr::BlockStmt(block_stmt) =
                    &arrow_expr.body
                  {
                    block_stmt.visit_children_with(a);
                    a.check_getter(block_stmt.span, prop.span());
                  }
                }
              });
            }
          } else if let Prop::Method(method_prop) = &**prop_expr {
            // e.g. Object.defineProperty(foo, 'bar', { get() {} })
            if let PropName::Ident(ident) = &method_prop.key {
              if ident.sym != *"get" {
                return;
              }

              self.visit_getter_or_function(|a| {
                a.set_getter_name(&method_prop.key);

                if let Some(body) = &method_prop.function.body {
                  body.visit_children_with(a);
                  a.check_getter(body.span, prop.span());
                }
              });
            }
          }
        }
      }
    }
  }

  fn visit_return_stmt(&mut self, return_stmt: &ReturnStmt, _: &dyn Node) {
    if self.getter_name.is_some() {
      self.has_return = true;
      if return_stmt.arg.is_none() {
        self.report_expected(return_stmt.span);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.9.0/tests/lib/rules/getter-return.js
  // MIT Licensed.

  #[test]
  fn getter_return_valid() {
    assert_lint_ok! {
      GetterReturn,
      "let foo = { get bar() { return true; } };",
      "class Foo { get bar() { return true; } }",
      "class Foo { bar() {} }",
      "class Foo { get bar() { if (baz) { return true; } else { return false; } } }",
      "class Foo { get() { return true; } }",
      r#"Object.defineProperty(foo, "bar", { get: function () { return true; } });"#,
      r#"Object.defineProperty(foo, "bar",
         { get: function () { ~function() { return true; }(); return true; } });"#,
      r#"Object.defineProperties(foo,
         { bar: { get: function() { return true; } } });"#,
      r#"Object.defineProperties(foo,
         { bar: { get: function () { ~function() { return true; }(); return true; } } });"#,
      "let get = function() {};",
      "let get = function() { return true; };",
      "let foo = { bar() {} };",
      "let foo = { bar() { return true; } };",
      "let foo = { bar: function() {} };",
      "let foo = { bar: function() { return; } };",
      "let foo = { bar: function() { return true; } };",
      "let foo = { get: function() {} };",
      "let foo = { get: () => {} };",
      r#"
const foo = {
  get getter() {
    const bar = {
      get getter() {
        return true;
      }
    };
    return 42;
  }
};
"#,
      r#"
class Foo {
  get foo() {
    class Bar {
      get bar() {
        return true;
      }
    };
    return 42;
  }
}
"#,
      r#"
Object.defineProperty(foo, 'bar', {
  get: function() {
    Object.defineProperty(x, 'y', {
      get: function() {
        return true;
      }
    });
    return 42;
  }
});
      "#,

      // https://github.com/denoland/deno_lint/issues/348
      r#"
const obj = {
  get root() {
    let primary = this;
    while (true) {
      if (primary.parent !== undefined) {
          primary = primary.parent;
      } else {
          return primary;
      }
    }
  }
};
      "#,

      // https://github.com/denoland/deno_lint/issues/462
      r#"
const obj = {
  get body() {
    if (foo) {
      return 1;
    } else {
      doSomething();
    }
    return 0;
  }
}
      "#,
      r#"
const obj = {
  get body() {
    if (this._stream) {
      return this._stream;
    }

    if (!this._bodySource) {
      return null;
    } else if (this._bodySource instanceof ReadableStream) {
      this._stream = this._bodySource;
    } else {
      const buf = bodyToArrayBuffer(this._bodySource);
      if (!(buf instanceof ArrayBuffer)) {
        throw new Error(
          `Expected ArrayBuffer from body`,
        );
      }

      this._stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new Uint8Array(buf));
          controller.close();
        },
      });
    }

    return this._stream;
  }
};
      "#
    };
  }

  #[test]
  fn getter_return_invalid() {
    assert_lint_err! {
      GetterReturn,

      // object getter
      "const foo = { get getter() {} };": [
        {
          col: 14,
          message: variant!(GetterReturnMessage, Expected, "getter"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { ~function() { return true; } } };": [
        {
          col: 14,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { function f() { return true; } } };": [
        {
          col: 14,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { const f = () => { return true; }; } };": [
        {
          col: 14,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { if (baz) { return true; } } };": [
        {
          col: 14,
          message: variant!(GetterReturnMessage, ExpectedAlways, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { return; } };": [
        {
          col: 26,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],

      // class getter
      "class Foo { get bar() {} }": [
        {
          col: 12,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = class { static get bar() {} }": [
        {
          col: 20,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "class Foo { get bar(){ if (baz) { return true; } } }": [
        {
          col: 12,
          message: variant!(GetterReturnMessage, ExpectedAlways, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      "class Foo { get bar(){ ~function () { return true; }() } }": [
        {
          col: 12,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],

      // Object.defineProperty
      "Object.defineProperty(foo, 'bar', { get: function(){} });": [
        {
          col: 36,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get: function getfoo(){} });": [
        {
          col: 36,
          message: variant!(GetterReturnMessage, Expected, "getfoo"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get(){} });": [
        {
          col: 36,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get: () => {} });": [
        {
          col: 36,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"Object.defineProperty(foo, "bar", { get: function() { if(bar) { return true; } } });"#: [
        {
          col: 36,
          message: variant!(GetterReturnMessage, ExpectedAlways, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"Object.defineProperty(foo, "bar", { get: function(){ ~function() { return true; }() } });"#: [
        {
          col: 36,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],

      // optional chaining
      r#"Object?.defineProperty(foo, 'bar', { get: function(){} });"#: [
        {
          col: 37,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"(Object?.defineProperty)(foo, 'bar', { get: function(){} });"#: [
        {
          col: 39,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],

      // nested
      r#"
const foo = {
  get getter() {
    const bar = {
      get getter() {}
    };
    return 42;
  }
};
      "#: [
        {
          line: 5,
          col: 6,
          message: variant!(GetterReturnMessage, Expected, "getter"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"
class Foo {
  get foo() {
    class Bar {
      get bar() {}
    };
    return 42;
  }
}
      "#: [
        {
          line: 5,
          col: 6,
          message: variant!(GetterReturnMessage, Expected, "bar"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"
Object.defineProperty(foo, 'bar', {
  get: function() {
    Object.defineProperty(x, 'y', {
      get: function() {}
    });
    return 42;
  }
});
      "#: [
        {
          line: 5,
          col: 6,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],

      // other
      "class b { get getterA() {} private get getterB() {} }": [
        {
          col: 10,
          message: variant!(GetterReturnMessage, Expected, "getterA"),
          hint: GetterReturnHint::Return,
        },
        {
          col: 27,
          message: variant!(GetterReturnMessage, Expected, "getterB"),
          hint: GetterReturnHint::Return,
        }
      ]
    };
  }
}
