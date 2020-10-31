// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::KeyDisplay;
use derive_more::Display;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::mem;
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  BlockStmtOrExpr, CallExpr, ClassMethod, Expr, ExprOrSuper, GetterProp,
  MethodKind, PrivateMethod, Prop, PropName, PropOrSpread, ReturnStmt,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct GetterReturn;

const CODE: &str = "getter-return";

#[derive(Display)]
enum GetterReturnMessage<'a> {
  #[display(fmt = "Expected to return a value in '{}'.", _0)]
  Expected(&'a dyn Display),
  #[display(fmt = "Expected '{}' to always return a value.", _0)]
  ExpectedAlways(&'a dyn Display),
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

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = GetterReturnVisitor::new(context);
    visitor.visit_program(program, program);
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

struct GetterReturnVisitor<'c, 'a> {
  context: &'c mut Context,
  errors: BTreeMap<Span, GetterReturnMessage<'a>>,
  /// If this visitor is currently in a getter, its name is stored.
  getter_name: Option<&'a dyn Display>,
  // `true` if a getter contains as least one return statement.
  has_return: bool,
}

impl<'c, 'a> GetterReturnVisitor<'c, 'a>
where
  'c: 'a,
{
  fn new(context: &'c mut Context) -> Self {
    Self {
      context,
      errors: BTreeMap::new(),
      getter_name: None,
      has_return: false,
    }
  }

  fn report(&'a mut self) {
    for (span, msg) in &self.errors {
      self.context.add_diagnostic_with_hint(
        *span,
        CODE,
        msg,
        GetterReturnHint::Return,
      );
    }
  }

  fn report_expected(&'a mut self, span: Span) {
    self.errors.insert(
      span,
      GetterReturnMessage::Expected(
        &self.getter_name.expect("the name of getter is not set"),
      ),
    );
  }

  fn report_always_expected(&'a mut self, span: Span) {
    self.errors.insert(
      span,
      GetterReturnMessage::ExpectedAlways(
        &self.getter_name.expect("the name of getter is not set"),
      ),
    );
  }

  fn check_getter(&'a mut self, getter_body_span: Span, getter_span: Span) {
    if self.getter_name.is_none() {
      return;
    }

    if self
      .context
      .control_flow
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

  fn set_getter_name<T: KeyDisplay>(&'a mut self, name: &'a T) {
    self.getter_name =
      Some(name.get_key_ref().unwrap_or(&"get") as &'a dyn Display);
  }

  fn set_default_getter_name(&'a mut self) {
    self.getter_name = Some(&"get");
  }

  fn visit_getter<F>(&'a mut self, op: F)
  where
    F: FnOnce(&'a mut Self),
  {
    let prev_name = mem::take(&mut self.getter_name);
    let prev_has_return = self.has_return;
    op(self);
    self.getter_name = prev_name;
    self.has_return = prev_has_return;
  }
}

impl<'c, 'a> Visit for GetterReturnVisitor<'c, 'a>
where
  'c: 'a,
{
  noop_visit_type!();

  fn visit_class_method(&mut self, class_method: &ClassMethod, _: &dyn Node) {
    self.visit_getter(|a| {
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
    self.visit_getter(|a| {
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
    self.visit_getter(|a| {
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

              self.visit_getter(|a| {
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

              self.visit_getter(|a| {
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
          message: GetterReturnMessage::Expected("getter".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { ~function() { return true; } } };": [
        {
          col: 14,
          // TODO(magurotuna): thie message should be `Expected`
          message: GetterReturnMessage::ExpectedAlways("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { if (baz) { return true; } } };": [
        {
          col: 14,
          message: GetterReturnMessage::ExpectedAlways("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = { get bar() { return; } };": [
        {
          col: 26,
          message: GetterReturnMessage::Expected("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],

      // class getter
      "class Foo { get bar() {} }": [
        {
          col: 12,
          message: GetterReturnMessage::Expected("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "const foo = class { static get bar() {} }": [
        {
          col: 20,
          message: GetterReturnMessage::Expected("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "class Foo { get bar(){ if (baz) { return true; } } }": [
        {
          col: 12,
          message: GetterReturnMessage::ExpectedAlways("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "class Foo { get bar(){ ~function () { return true; }() } }": [
        {
          col: 12,
          // TODO(magurotuna): thie message should be `Expected`
          message: GetterReturnMessage::ExpectedAlways("bar".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],

      // Object.defineProperty
      "Object.defineProperty(foo, 'bar', { get: function(){} });": [
        {
          col: 36,
          message: GetterReturnMessage::Expected("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get: function getfoo(){} });": [
        {
          col: 36,
          message: GetterReturnMessage::Expected("getfoo".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get(){} });": [
        {
          col: 36,
          message: GetterReturnMessage::Expected("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperty(foo, 'bar', { get: () => {} });": [
        {
          col: 36,
          message: GetterReturnMessage::Expected("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"Object.defineProperty(foo, "bar", { get: function() { if(bar) { return true; } } });"#: [
        {
          col: 36,
          message: GetterReturnMessage::ExpectedAlways("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"Object.defineProperty(foo, "bar", { get: function(){ ~function() { return true; }() } });"#: [
        {
          col: 36,
          // TODO(magurotuna): thie message should be `Expected`
          message: GetterReturnMessage::ExpectedAlways("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],

      // optional chaining
      r#"Object?.defineProperty(foo, 'bar', { get: function(){} });"#: [
        {
          col: 37,
          message: GetterReturnMessage::Expected("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"(Object?.defineProperty)(foo, 'bar', { get: function(){} });"#: [
        {
          col: 39,
          message: GetterReturnMessage::Expected("get".to_string()),
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
          message: GetterReturnMessage::Expected("getter".to_string()),
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
          message: GetterReturnMessage::Expected("bar".to_string()),
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
          message: GetterReturnMessage::Expected("get".to_string()),
          hint: GetterReturnHint::Return,
        }
      ],

      // other
      "class b { get getterA() {} private get getterB() {} }": [
        {
          col: 10,
          message: GetterReturnMessage::Expected("getterA".to_string()),
          hint: GetterReturnHint::Return,
        },
        {
          col: 27,
          message: GetterReturnMessage::Expected("getterB".to_string()),
          hint: GetterReturnHint::Return,
        }
      ]
    };
  }
}
