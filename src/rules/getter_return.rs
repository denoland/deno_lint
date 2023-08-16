// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::program_ref;
use super::{Context, LintRule};
use crate::swc_util::StringRepr;
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::{
  ArrowExpr, BlockStmtOrExpr, CallExpr, Callee, ClassMethod, Expr,
  ExprOrSpread, FnDecl, FnExpr, GetterProp, MemberExpr, MemberProp, MethodKind,
  MethodProp, ObjectLit, OptCall, OptChainBase, PrivateMethod, Prop, PropName,
  PropOrSpread, ReturnStmt,
};
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Visit;
use deno_ast::swc::visit::VisitWith;
use deno_ast::SourceRange;
use deno_ast::SourceRangedForSpanned;
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
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = GetterReturnVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
    visitor.report();
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/getter_return.md")
  }
}

struct GetterReturnVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  errors: BTreeMap<SourceRange, GetterReturnMessage>,
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
    for (range, msg) in &self.errors {
      self.context.add_diagnostic_with_hint(
        *range,
        CODE,
        msg,
        GetterReturnHint::Return,
      );
    }
  }

  fn report_expected(&mut self, range: SourceRange) {
    self.errors.insert(
      range,
      GetterReturnMessage::Expected(
        self
          .getter_name
          .clone()
          .expect("the name of getter is not set"),
      ),
    );
  }

  fn report_always_expected(&mut self, range: SourceRange) {
    self.errors.insert(
      range,
      GetterReturnMessage::ExpectedAlways(
        self
          .getter_name
          .clone()
          .expect("the name of getter is not set"),
      ),
    );
  }

  fn check_getter(
    &mut self,
    getter_body_range: SourceRange,
    getter_range: SourceRange,
  ) {
    if self.getter_name.is_none() {
      return;
    }

    if self
      .context
      .control_flow()
      .meta(getter_body_range.start)
      .unwrap()
      .continues_execution()
    {
      if self.has_return {
        self.report_always_expected(getter_range);
      } else {
        self.report_expected(getter_range);
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

  fn check_callee_expr<F>(&mut self, expr: &Expr, op: F)
  where
    F: FnOnce(&mut Self, &MemberExpr),
  {
    match expr {
      Expr::Paren(paren) => {
        self.check_callee_expr(&paren.expr, op);
      }
      Expr::Member(member) => {
        op(self, member);
      }
      Expr::OptChain(opt) => {
        if let OptChainBase::Member(member) = &*opt.base {
          op(self, member);
        }
      }
      _ => {}
    }
  }

  fn check_obj_method_getter_return(&mut self, obj_expr: &ObjectLit) {
    for prop in obj_expr.props.iter() {
      if let PropOrSpread::Prop(prop_expr) = prop {
        if let Prop::KeyValue(kv_prop) = &**prop_expr {
          if let Expr::Object(obj_expr) = &*kv_prop.value {
            // e.g. Object.create(foo, { bar: { get: function() {} } })
            self.check_obj_method_getter_return(obj_expr);
            continue;
          }

          // e.g. Object.defineProperty(foo, 'bar', { get: function() {} })
          if let PropName::Ident(ident) = &kv_prop.key {
            if ident.sym != *"get" {
              continue;
            }
            self.visit_getter_or_function(|a| {
              // function
              if let Expr::Fn(fn_expr) = &*kv_prop.value {
                if fn_expr.function.is_generator {
                  return;
                }
                a.set_getter_name(&fn_expr.ident);
                if let Some(body) = &fn_expr.function.body {
                  body.visit_children_with(a);
                  a.check_getter(body.range(), prop.range());
                }
                // arrow function
              } else if let Expr::Arrow(arrow_expr) = &*kv_prop.value {
                a.set_default_getter_name();
                if let BlockStmtOrExpr::BlockStmt(block_stmt) =
                  &*arrow_expr.body
                {
                  block_stmt.visit_children_with(a);
                  a.check_getter(block_stmt.range(), prop.range());
                }
              }
            });
          }
        } else if let Prop::Method(method_prop) = &**prop_expr {
          if method_prop.function.is_generator {
            return;
          }
          // e.g. Object.defineProperty(foo, 'bar', { get() {} })
          if let PropName::Ident(ident) = &method_prop.key {
            if ident.sym != *"get" {
              continue;
            }
            self.visit_getter_or_function(|a| {
              a.set_getter_name(&method_prop.key);
              if let Some(body) = &method_prop.function.body {
                body.visit_children_with(a);
                a.check_getter(body.range(), prop.range());
              }
            });
          }
        }
      }
    }
  }

  fn check_call_expr(&mut self, callee_expr: &Expr, args: &[ExprOrSpread]) {
    if !(matches!(args.len(), 2 | 3)) {
      return;
    }

    self.check_callee_expr(callee_expr, |visitor, member_expr| {
      if let Expr::Ident(ident) = &*member_expr.obj {
        if !(matches!(ident.sym.as_ref(), "Object" | "Reflect")) {
          return;
        }

        if let MemberProp::Ident(ident) = &member_expr.prop {
          if !(matches!(
            ident.sym.as_ref(),
            "create" | "defineProperty" | "defineProperties"
          )) {
            return;
          }
        }
      }

      if let Expr::Object(obj_expr) = &*args[args.len() - 1].expr {
        visitor.check_obj_method_getter_return(obj_expr)
      }
    });
  }
}

impl<'c, 'view> Visit for GetterReturnVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
    // `self.has_return` should be reset because return statements inside the `fn_decl` don't have
    // effect on outside of it
    self.visit_getter_or_function(|a| {
      fn_decl.visit_children_with(a);
    });
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    // `self.has_return` should be reset because return statements inside the `fn_expr` don't have
    // effect on outside of it
    self.visit_getter_or_function(|a| {
      fn_expr.visit_children_with(a);
    });
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    // `self.has_return` should be reset because return statements inside the `arrow_expr` don't
    // have effect on outside of it
    self.visit_getter_or_function(|a| {
      arrow_expr.visit_children_with(a);
    });
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp) {
    // `self.has_return` should be reset because return statements inside the `method_prop` don't
    // have effect on outside of it
    self.visit_getter_or_function(|a| {
      method_prop.visit_children_with(a);
    });
  }
  fn visit_class_method(&mut self, class_method: &ClassMethod) {
    self.visit_getter_or_function(|a| {
      if class_method.kind == MethodKind::Getter {
        a.set_getter_name(&class_method.key);
      }
      class_method.visit_children_with(a);

      if let Some(body) = &class_method.function.body {
        a.check_getter(body.range(), class_method.range());
      }
    });
  }

  fn visit_private_method(&mut self, private_method: &PrivateMethod) {
    self.visit_getter_or_function(|a| {
      if private_method.kind == MethodKind::Getter {
        a.set_getter_name(&private_method.key);
      }
      private_method.visit_children_with(a);

      if let Some(body) = &private_method.function.body {
        a.check_getter(body.range(), private_method.range());
      }
    });
  }

  fn visit_getter_prop(&mut self, getter_prop: &GetterProp) {
    self.visit_getter_or_function(|a| {
      a.set_getter_name(&getter_prop.key);
      getter_prop.visit_children_with(a);

      if let Some(body) = &getter_prop.body {
        a.check_getter(body.range(), getter_prop.range());
      }
    });
  }

  fn visit_opt_call(&mut self, opt_call: &OptCall) {
    opt_call.visit_children_with(self);
    self.check_call_expr(&opt_call.callee, &opt_call.args);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr) {
    call_expr.visit_children_with(self);
    let Callee::Expr(callee_expr) = &call_expr.callee else {
      return;
    };
    self.check_call_expr(callee_expr, &call_expr.args);
  }

  fn visit_return_stmt(&mut self, return_stmt: &ReturnStmt) {
    if self.getter_name.is_some() {
      self.has_return = true;
      if return_stmt.arg.is_none() {
        self.report_expected(return_stmt.range());
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
