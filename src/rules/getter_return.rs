// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::{GetSpan, Span};
use derive_more::Display;
use std::collections::BTreeMap;

#[derive(Debug)]
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
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut visitor = GetterReturnVisitor::new(context);
    visitor.visit_program(program);
    visitor.report();
  }
}

struct GetterReturnVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  errors: BTreeMap<Span, GetterReturnMessage>,
  /// If this visitor is currently in a getter, its name is stored.
  getter_name: Option<String>,
  // `true` if a getter contains at least one return statement.
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

    let continues =
      match self.context.control_flow().meta(getter_body_span.start) {
        Some(meta) => meta.continues_execution(),
        // No entry in control flow means the scope had no explicit termination
        // tracked (e.g. empty function body), so execution continues.
        None => true,
      };
    if continues {
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
    self.has_return = false;
    op(self);
    self.getter_name = prev_name;
    self.has_return = prev_has_return;
  }

  fn check_callee_expr<F>(&mut self, expr: &Expression, op: F)
  where
    F: FnOnce(&mut Self, &StaticMemberExpression),
  {
    match expr {
      Expression::ParenthesizedExpression(paren) => {
        self.check_callee_expr(&paren.expression, op);
      }
      Expression::StaticMemberExpression(member) => {
        op(self, member);
      }
      Expression::ChainExpression(chain) => {
        // Handle optional chaining like `Object?.defineProperty`
        if let ChainElement::StaticMemberExpression(member) = &chain.expression
        {
          op(self, member);
        }
      }
      Expression::ComputedMemberExpression(_) => {}
      Expression::PrivateFieldExpression(_) => {}
      _ => {}
    }
  }

  fn check_obj_method_getter_return(
    &mut self,
    obj_expr: &ObjectExpression<'view>,
  ) {
    for prop in &obj_expr.properties {
      let ObjectPropertyKind::ObjectProperty(prop_expr) = prop else {
        continue;
      };

      // Check for nested objects
      if let Expression::ObjectExpression(obj_expr) = &prop_expr.value {
        self.check_obj_method_getter_return(obj_expr);
        continue;
      }

      let Some(key_name) = prop_expr.key.string_repr() else {
        continue;
      };
      if key_name != "get" {
        continue;
      }

      if prop_expr.method {
        // e.g. Object.defineProperty(foo, 'bar', { get() {} })
        if let Expression::FunctionExpression(fn_expr) = &prop_expr.value {
          if fn_expr.generator {
            continue;
          }
          let fn_span = fn_expr.span;
          self.visit_getter_or_function(|a| {
            a.set_getter_name(&prop_expr.key);
            if let Some(body) = &fn_expr.body {
              walk::walk_function_body(a, body);
              a.check_getter(fn_span, prop.span());
            }
          });
        }
      } else {
        // e.g. Object.defineProperty(foo, 'bar', { get: function() {} })
        match &prop_expr.value {
          Expression::FunctionExpression(fn_expr) => {
            if fn_expr.generator {
              continue;
            }
            let fn_span = fn_expr.span;
            self.visit_getter_or_function(|a| {
              if let Some(id) = &fn_expr.id {
                a.getter_name = Some(id.name.to_string());
              } else {
                a.set_default_getter_name();
              }
              if let Some(body) = &fn_expr.body {
                walk::walk_function_body(a, body);
                a.check_getter(fn_span, prop.span());
              }
            });
          }
          Expression::ArrowFunctionExpression(arrow_expr) => {
            // If arrow has expression body, it always returns
            if arrow_expr.expression {
              continue;
            }
            let arrow_span = arrow_expr.span;
            self.visit_getter_or_function(|a| {
              a.set_default_getter_name();
              walk::walk_function_body(a, &arrow_expr.body);
              a.check_getter(arrow_span, prop.span());
            });
          }
          _ => {}
        }
      }
    }
  }

  fn check_call_expr(
    &mut self,
    callee_expr: &Expression<'view>,
    args: &[Argument<'view>],
  ) {
    if !(matches!(args.len(), 2 | 3)) {
      return;
    }

    self.check_callee_expr(callee_expr, |visitor, member_expr| {
      if let Expression::Identifier(ident) = &member_expr.object {
        if !(matches!(ident.name.as_str(), "Object" | "Reflect")) {
          return;
        }

        if !(matches!(
          member_expr.property.name.as_str(),
          "create" | "defineProperty" | "defineProperties"
        )) {
          return;
        }
      } else {
        return;
      }

      if let Argument::ObjectExpression(obj_expr) = &args[args.len() - 1] {
        visitor.check_obj_method_getter_return(obj_expr)
      }
    });
  }
}

impl<'a> Visit<'a> for GetterReturnVisitor<'_, 'a> {
  fn visit_function(
    &mut self,
    func: &Function<'a>,
    flags: deno_ast::oxc::syntax::scope::ScopeFlags,
  ) {
    // `self.has_return` should be reset because return statements inside
    // don't have effect on outside of it
    self.visit_getter_or_function(|a| {
      walk::walk_function(a, func, flags);
    });
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow_expr: &ArrowFunctionExpression<'a>,
  ) {
    // `self.has_return` should be reset because return statements inside
    // don't have effect on outside of it
    self.visit_getter_or_function(|a| {
      walk::walk_arrow_function_expression(a, arrow_expr);
    });
  }

  fn visit_method_definition(&mut self, method_def: &MethodDefinition<'a>) {
    if method_def.kind == MethodDefinitionKind::Get {
      let fn_span = method_def.value.span;
      let method_span = method_def.span;
      self.visit_getter_or_function(|a| {
        a.set_getter_name(&method_def.key);
        if let Some(body) = &method_def.value.body {
          walk::walk_function_body(a, body);
        }
        a.check_getter(fn_span, method_span);
      });
    } else {
      walk::walk_method_definition(self, method_def);
    }
  }

  fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
    if prop.kind == PropertyKind::Get {
      if let Expression::FunctionExpression(func) = &prop.value {
        let prop_span = prop.span;
        let fn_span = func.span;
        self.visit_getter_or_function(|a| {
          a.set_getter_name(&prop.key);
          if let Some(body) = &func.body {
            walk::walk_function_body(a, body);
          }
          a.check_getter(fn_span, prop_span);
        });
      } else {
        walk::walk_object_property(self, prop);
      }
    } else {
      walk::walk_object_property(self, prop);
    }
  }

  fn visit_call_expression(&mut self, call_expr: &CallExpression<'a>) {
    walk::walk_call_expression(self, call_expr);
    self.check_call_expr(&call_expr.callee, &call_expr.arguments);
  }

  fn visit_return_statement(&mut self, return_stmt: &ReturnStatement<'a>) {
    if self.getter_name.is_some() {
      self.has_return = true;
      if return_stmt.argument.is_none() {
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
      r#"Reflect.defineProperty(foo, "bar", { get: function () { return true; } });"#,
      r#"Reflect.defineProperty(foo, "bar", { get: () => { return true; } });"#,
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
      "#,
      r#"
class _Test {
  get foo() {
    const target = {
      asd() {
        asd;
        return;
      },
    };
    return target;
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/1088
      "Object.create(foo)",
      "Object.create(foo, { bar: { configurable: false, get: () => { return true } } })",
      "Object.create(foo, { bar: { configurable: false, get: function() { return true } } })",
      "Object.create(foo, { bar: { configurable: false, get() { return true } } })",
      r#"
Object.create(Object.prototype, {
  foo: {
    writable: true,
    configurable: true,
    value: 'hello'
  },
  bar: {
    configurable: false,
    get: function() { return value },
    set: function(value) {
      console.log('Setting `o.bar` to', value);
    }
  }
})
      "#,

      // https://github.com/denoland/deno_lint/issues/1072
      r#"
Object.defineProperty(Number.prototype, x, {
  *get() {
    for (let n = 0; n < this; n++) yield n;
  },
});
      "#,
      r#"
Object.defineProperty(Number.prototype, x, {
  get: function* () {
    for (let n = 0; n < this; n++) yield n;
  },
});
      "#,
    }
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

      // Object.create
      "Object.create(foo, { bar: { configurable: false, get: () => {} } })": [
        {
          col: 49,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.create(foo, { bar: { configurable: false, get: function() {} } })": [
        {
          col: 49,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],

      "Object.create(foo, { bar: { configurable: false, get: function() {} } })": [
        {
          col: 49,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"
Object.create(Object.prototype, {
  foo: {
    writable: true,
    configurable: true,
    value: 'hello'
  },
  bar: {
    configurable: false,
    get: function() {},
    set: function(value) {
      console.log('Setting `o.bar` to', value);
    }
  }
})
      "#: [
        {
          line: 10,
          col: 4,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],

      // Object.defineProperties
      "Object.defineProperties(obj, { 'property1': { value: true, writable: true, get: () => {} } });": [
        {
          col: 75,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperties(obj, { 'property1': { value: true, writable: true, get: function(){} } });": [
        {
          col: 75,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Object.defineProperties(obj, { 'property1': { value: true, writable: true, get(){} } });": [
        {
          col: 75,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"
Object.defineProperties(obj, {
  'property1': {
    value: true,
    writable: true,
    get: () => {}
  },
  'property2': {
    value: 'Hello',
    writable: false,
    get: () => { return true }
  }
});
      "#: [
        {
          line: 6,
          col: 4,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      r#"
Object.defineProperties(obj, {
  'property1': {
    value: true,
    writable: true,
    get: () => { return true }
  },
  'property2': {
    value: 'Hello',
    writable: false,
    get: () => {}
  }
});
      "#: [
        {
          line: 11,
          col: 4,
          message: variant!(GetterReturnMessage, Expected, "get"),
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

      // Reflect.defineProperty
      "Reflect.defineProperty(foo, 'bar', { get: function(){} });": [
        {
          col: 37,
          message: variant!(GetterReturnMessage, Expected, "get"),
          hint: GetterReturnHint::Return,
        }
      ],
      "Reflect.defineProperty(foo, 'bar', { get: () => {} });": [
        {
          col: 37,
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
