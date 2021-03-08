// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait, Spanned};

pub struct NoSetterReturn;

impl LintRule for NoSetterReturn {
  fn new() -> Box<Self> {
    Box::new(NoSetterReturn)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-setter-return"
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!()
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    NoSetterReturnHandler.traverse(program, context);
  }
}

struct NoSetterReturnHandler;

impl Handler for NoSetterReturnHandler {
  fn return_stmt(&self, return_stmt: &AstView::ReturnStmt, ctx: &mut Context) {
    // return without a value is allowed
    if return_stmt.arg.is_none() {
      return;
    }

    fn inside_setter(node: AstView::Node) -> bool {
      use AstView::Node::*;
      match node {
        SetterProp(_) => true,
        ClassMethod(ref method) => {
          if method.kind() == AstView::MethodKind::Setter {
            true
          } else {
            false
          }
        }
        FnDecl(_) | FnExpr(_) | ArrowExpr(_) => false,
        _ => {
          if let Some(parent) = node.parent() {
            inside_setter(parent)
          } else {
            false
          }
        }
      }
    }

    if inside_setter(return_stmt.into_node()) {
      ctx.add_diagnostic(
        return_stmt.span(),
        "no-setter-return",
        "Setter cannot return a value",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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

      //------------------------------------------------------------------------------
      // Object literals and classes
      //------------------------------------------------------------------------------

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
      "({ set foo(val) { function foo(val) { return 1; } } })",
      "({ set foo(val) { var foo = function(val) { return 1; } } })",
      "({ set foo(val) { var foo = (val) => { return 1; } } })",
      "({ set foo(val) { var foo = (val) => 1; } })",
      "({ set [function() { return 1; }](val) {} })",
      "({ set [() => { return 1; }](val) {} })",
      "({ set [() => 1](val) {} })",
      "({ set foo(val = function() { return 1; }) {} })",
      "({ set foo(val = v => 1) {} })",
      "(class { set foo(val) { function foo(val) { return 1; } } })",
      "(class { set foo(val) { var foo = function(val) { return 1; } } })",
      "(class { set foo(val) { var foo = (val) => { return 1; } } })",
      "(class { set foo(val) { var foo = (val) => 1; } })",
      "(class { set [function() { return 1; }](val) {} })",
      "(class { set [() => { return 1; }](val) {} })",
      "(class { set [() => 1](val) {} })",
      "(class { set foo(val = function() { return 1; }) {} })",
      "(class { set foo(val = (v) => 1) {} })",

      //------------------------------------------------------------------------------
      // Property descriptors
      //------------------------------------------------------------------------------

      // return without a value is allowed
      "Object.defineProperty(foo, 'bar', { set(val) { return; } })",
      "Reflect.defineProperty(foo, 'bar', { set(val) { if (val) { return; } } })",
      "Object.defineProperties(foo, { bar: { set(val) { try { return; } catch(e){} } } })",
      "Object.create(foo, { bar: { set: function(val) { return; } } })",

      // not a setter
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
    assert_lint_err::<NoSetterReturn>(
      r#"const a = { set setter(a) { return "something"; } };"#,
      28,
    );
    assert_lint_err_on_line_n::<NoSetterReturn>(
      r#"
class b {
  set setterA(a) {
    return "something";
  }
  private set setterB(a) {
    return "something";
  }
}
      "#,
      vec![(4, 4), (7, 4)],
    );
  }
}
