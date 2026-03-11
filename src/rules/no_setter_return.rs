// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::syntax::scope::ScopeFlags;

#[derive(Debug)]
pub struct NoSetterReturn;

const CODE: &str = "no-setter-return";
const MESSAGE: &str = "Setter cannot return a value";

impl LintRule for NoSetterReturn {
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
    let mut visitor = NoSetterReturnVisitor {
      context,
      in_setter_stack: vec![],
    };
    visitor.visit_program(program);
  }
}

struct NoSetterReturnVisitor<'c, 'a> {
  context: &'c mut Context<'a>,
  /// Stack tracking whether each function boundary is a setter.
  in_setter_stack: Vec<bool>,
}

impl NoSetterReturnVisitor<'_, '_> {
  fn is_in_setter(&self) -> bool {
    self.in_setter_stack.last().copied().unwrap_or(false)
  }
}

impl<'a> Visit<'a> for NoSetterReturnVisitor<'_, 'a> {
  fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
    // Walk the computed key (if any) without setter context
    if let Some(key_expr) = method.key.as_expression() {
      self.visit_expression(key_expr);
    }
    // Walk the function value with appropriate context
    let is_setter = method.kind == MethodDefinitionKind::Set;
    self.in_setter_stack.push(is_setter);
    walk::walk_function(self, &method.value, ScopeFlags::empty());
    self.in_setter_stack.pop();
  }

  fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
    // Walk the key without setter context (computed keys may contain functions)
    if let Some(expr) = prop.key.as_expression() {
      self.visit_expression(expr);
    }
    // Walk the value with appropriate context
    let is_setter = prop.kind == PropertyKind::Set;
    if is_setter {
      if let Expression::FunctionExpression(func) = &prop.value {
        self.in_setter_stack.push(true);
        walk::walk_function(self, func, ScopeFlags::empty());
        self.in_setter_stack.pop();
        return;
      }
    }
    self.visit_expression(&prop.value);
  }

  fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
    // Regular function (not via method/object_property): not a setter
    self.in_setter_stack.push(false);
    walk::walk_function(self, func, flags);
    self.in_setter_stack.pop();
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    self.in_setter_stack.push(false);
    walk::walk_arrow_function_expression(self, arrow);
    self.in_setter_stack.pop();
  }

  fn visit_return_statement(&mut self, return_stmt: &ReturnStatement<'a>) {
    if return_stmt.argument.is_some() && self.is_in_setter() {
      self
        .context
        .add_diagnostic(return_stmt.span, CODE, MESSAGE);
    }
    walk::walk_return_statement(self, return_stmt);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.21.0/tests/lib/rules/no-setter-return.js
  // MIT Licensed.

  #[test]
  fn no_setter_return_valid() {
    assert_lint_ok! {
      NoSetterReturn,
      "function foo() { return 1; }",
      "function set(val) { return 1; }",
      "var foo = function() { return 1; };",
      "var foo = function set() { return 1; };",
      "var set = function() { return 1; };",
      "var set = function set(val) { return 1; };",
      "var set = val => { return 1; };",
      "var set = val => 1;",
      "({ set a(val) { }}); function foo() { return 1; }",
      "({ set a(val) { }}); (function () { return 1; });",
      "({ set a(val) { }}); (() => { return 1; });",
      "({ set a(val) { }}); (() => 1);",

      // return without a value is allowed
      "({ set foo(val) { return; } })",
      "({ set foo(val) { if (val) { return; } } })",
      "class A { set foo(val) { return; } }",
      "(class { set foo(val) { if (val) { return; } else { return; } return; } })",
      "class A { set foo(val) { try {} catch(e) { return; } } }",

      // not a setter
      "({ get foo() { return 1; } })",
      "({ get set() { return 1; } })",
      "({ set(val) { return 1; } })",
      "({ set: function(val) { return 1; } })",
      "({ foo: function set(val) { return 1; } })",
      "({ set: function set(val) { return 1; } })",
      "({ set: (val) => { return 1; } })",
      "({ set: (val) => 1 })",
      "set = { foo(val) { return 1; } };",
      "class A { constructor(val) { return 1; } }",
      "class set { constructor(val) { return 1; } }",
      "class set { foo(val) { return 1; } }",
      "var set = class { foo(val) { return 1; } }",
      "(class set { foo(val) { return 1; } })",
      "class A { get foo() { return val; } }",
      "class A { get set() { return val; } }",
      "class A { set(val) { return 1; } }",
      "class A { static set(val) { return 1; } }",
      "({ set: set = function set(val) { return 1; } } = {})",
      "({ set: set = (val) => 1 } = {})",

      // not returning from the setter
      "({ set foo(val) { function b(){} return; } })",
      "({ set foo(val) { function b(){ return 1; } } })",
      "({ set foo(val) { var foo = function(val) { return 1; } } })",
      "({ set foo(val) { var foo = (val) => { return 1; } } })",
      "({ set foo(val) { var foo = (val) => 1; } })",
      "({ set [function() { return 1; }](val) {} })",
      "({ set [() => { return 1; }](val) {} })",
      "({ set [() => 1](val) {} })",
      "({ set foo(val = function() { return 1; }) {} })",
      "({ set foo(val = v => 1) {} })",
      "(class { set foo(val) { function b(){ return 1; } } })",
      "(class { set foo(val) { var foo = function(val) { return 1; } } })",
      "(class { set foo(val) { var foo = (val) => { return 1; } } })",
      "(class { set foo(val) { var foo = (val) => 1; } })",
      "(class { set [function() { return 1; }](val) {} })",
      "(class { set [() => { return 1; }](val) {} })",
      "(class { set [() => 1](val) {} })",
      "(class { set foo(val = function() { return 1; }) {} })",
      "(class { set foo(val = (v) => 1) {} })",

      // Property descriptors - return without value
      "Object.defineProperty(foo, 'bar', { set(val) { return; } })",
      "Reflect.defineProperty(foo, 'bar', { set(val) { if (val) { return; } } })",
      "Object.defineProperties(foo, { bar: { set(val) { try { return; } catch(e){} } } })",
      "Object.create(foo, { bar: { set: function(val) { return; } } })",

      // Property descriptors - not a setter
      "x = { set(val) { return 1; } }",
      "x = { foo: { set(val) { return 1; } } }",
      "Object.defineProperty(foo, 'bar', { value(val) { return 1; } })",
      "Reflect.defineProperty(foo, 'bar', { value: function set(val) { return 1; } })",
      "Object.defineProperties(foo, { bar: { [set](val) { return 1; } } })",
      "Object.create(foo, { bar: { 'set ': function(val) { return 1; } } })",
      "Object.defineProperty(foo, 'bar', { [`set `]: (val) => { return 1; } })",
      "Reflect.defineProperty(foo, 'bar', { Set(val) { return 1; } })",
      "Object.defineProperties(foo, { bar: { value: (val) => 1 } })",
      "Object.create(foo, { set: { value: function(val) { return 1; } } })",
      "Object.defineProperty(foo, 'bar', { baz(val) { return 1; } })",
      "Reflect.defineProperty(foo, 'bar', { get(val) { return 1; } })",
      "Object.create(foo, { set: function(val) { return 1; } })",
      "Object.defineProperty(foo, { set: (val) => 1 })",

      // not returning from the setter
      "Object.defineProperty(foo, 'bar', { set(val) { function foo() { return 1; } } })",
      "Reflect.defineProperty(foo, 'bar', { set(val) { var foo = function() { return 1; } } })",
      "Object.defineProperties(foo, { bar: { set(val) { () => { return 1 }; } } })",
      "Object.create(foo, { bar: { set: (val) => { (val) => 1; } } })",
    };
  }

  #[test]
  fn no_setter_return_invalid() {
    assert_lint_err! {
      NoSetterReturn,
      r#"const a = { set setter(a) { return "something"; } };"#: [
        {
          col: 28,
          message: "Setter cannot return a value",
        }
      ],
      r#"
class b {
  set setterA(a) {
    return "something";
  }
  private set setterB(a) {
    return "something";
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
        },
        {
          line: 7,
          col: 4,
          message: MESSAGE,
        }
      ],
      "({ set a(val){ return val + 1; } })": [
        {
          col: 15,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { return 1; } })": [
        {
          col: 16,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { return 1; } }": [
        {
          col: 23,
          message: MESSAGE,
        }
      ],
      "class A { static set a(val) { return 1; } }": [
        {
          col: 30,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { return 1; } })": [
        {
          col: 22,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { return val; } })": [
        {
          col: 16,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { return undefined; } }": [
        {
          col: 23,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { return null; } })": [
        {
          col: 22,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { return x + y; } })": [
        {
          col: 16,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { return foo(); } }": [
        {
          col: 23,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { return this._a; } })": [
        {
          col: 22,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { return this.a; } })": [
        {
          col: 16,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { if (foo) { return 1; }; } })": [
        {
          col: 27,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { try { return 1; } catch(e) {} } }": [
        {
          col: 29,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { while (foo){ if (bar) break; else return 1; } } })": [
        {
          col: 56,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { return 1; }, set b(val) { return 1; } })": [
        {
          col: 16,
          message: MESSAGE,
        },
        {
          col: 42,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { return 1; } set b(val) { return 1; } }": [
        {
          col: 23,
          message: MESSAGE,
        },
        {
          col: 48,
          message: MESSAGE,
        },
      ],
      "(class { set a(val) { return 1; } static set b(val) { return 1; } })": [
        {
          col: 22,
          message: MESSAGE,
        },
        {
          col: 54,
          message: MESSAGE,
        },
      ],
      "({ set a(val) { if(val) { return 1; } else { return 2 }; } })": [
        {
          col: 26,
          message: MESSAGE,
        },
        {
          col: 45,
          message: MESSAGE,
        },
      ],
      "class A { set a(val) { switch(val) { case 1: return x; case 2: return y; default: return z } } }": [
        {
          col: 45,
          message: MESSAGE,
        },
        {
          col: 63,
          message: MESSAGE,
        },
        {
          col: 82,
          message: MESSAGE,
        },
      ],
      "(class { static set a(val) { if (val > 0) { this._val = val; return val; } return false; } })": [
        {
          col: 61,
          message: MESSAGE,
        },
        {
          col: 75,
          message: MESSAGE,
        },
      ],
      "({ set a(val) { if(val) { return 1; } else { return; }; } })": [
        {
          col: 26,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { switch(val) { case 1: return x; case 2: return; default: return z } } }": [
        {
          col: 45,
          message: MESSAGE,
        },
        {
          col: 80,
          message: MESSAGE,
        },
      ],
      "(class { static set a(val) { if (val > 0) { this._val = val; return; } return false; } })": [
        {
          col: 71,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { function b(){} return b(); } })": [
        {
          col: 31,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { return () => {}; } }": [
        {
          col: 23,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { function b(){ return 1; } return 2; } })": [
        {
          col: 48,
          message: MESSAGE,
        }
      ],
      "({ set a(val) { function b(){ return; } return 1; } })": [
        {
          col: 40,
          message: MESSAGE,
        }
      ],
      "class A { set a(val) { var x = function() { return 1; }; return 2; } }": [
        {
          col: 57,
          message: MESSAGE,
        }
      ],
      "(class { set a(val) { var x = () => { return; }; return 2; } })": [
        {
          col: 49,
          message: MESSAGE,
        }
      ],
      "function f(){}; ({ set a(val) { return 1; } });": [
        {
          col: 32,
          message: MESSAGE,
        }
      ],
      "x = function f(){}; class A { set a(val) { return 1; } };": [
        {
          col: 43,
          message: MESSAGE,
        }
      ],
      "x = () => {}; A = class { set a(val) { return 1; } };": [
        {
          col: 39,
          message: MESSAGE,
        }
      ],
      "return; ({ set a(val) { return 1; } }); return 2;": [
        {
          col: 24,
          message: MESSAGE,
        }
      ],
    };
  }
}
