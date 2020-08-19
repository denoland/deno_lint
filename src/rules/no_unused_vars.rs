// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::VarDeclarator;
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

use std::collections::HashSet;
use std::sync::Arc;

pub struct NoUnusedVars;

impl LintRule for NoUnusedVars {
  fn new() -> Box<Self> {
    Box::new(NoUnusedVars)
  }

  fn code(&self) -> &'static str {
    "no-unused-vars"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = Collector {
      used_vars: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoUnusedVarVisitor::new(context, collector.used_vars);
    module.visit_with(module, &mut visitor);
  }
}

/// Collects information about variable usages.
struct Collector {
  used_vars: HashSet<Id>,
}

impl Visit for Collector {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc
  // It will make binary much smaller. In case of swc, binary size is reduced to 18mb from 29mb.
  // noop_visit_type!();

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    match expr {
      Expr::Ident(i) => {
        // Mark the variable as used.
        self.used_vars.insert(i.to_id());
      }
      _ => expr.visit_children_with(self),
    }
  }

  fn visit_pat(&mut self, pat: &Pat, _: &dyn Node) {
    match pat {
      // Ignore patterns
      Pat::Ident(..) | Pat::Invalid(..) => {}
      //
      _ => pat.visit_children_with(self),
    }
  }

  fn visit_member_expr(&mut self, member_expr: &MemberExpr, _: &dyn Node) {
    member_expr.obj.visit_with(member_expr, self);
    if member_expr.computed {
      member_expr.prop.visit_with(member_expr, self);
    }
  }
}

struct NoUnusedVarVisitor {
  context: Arc<Context>,
  used_vars: HashSet<Id>,
}

impl NoUnusedVarVisitor {
  fn new(context: Arc<Context>, used_vars: HashSet<Id>) -> Self {
    Self { context, used_vars }
  }
}

/// As we only care about variables, only variable declrations are checked.
impl Visit for NoUnusedVarVisitor {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&declarator.name);

    for ident in declared_idents {
      if !self.used_vars.contains(&ident.to_id()) {
        // The variable is not used.
        self.context.add_diagnostic(
          ident.span,
          "no-unused-vars",
          &format!("\"{}\" label is never used", ident.sym),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unused_vars_ok() {
    assert_lint_ok::<NoUnusedVars>("var a = 1; console.log(a)");
    assert_lint_ok::<NoUnusedVars>(
      "var a = 1; function foo() { console.log(a) } ",
    );
    assert_lint_ok::<NoUnusedVars>(
      "var a = 1; const arrow = () => a; console.log(arrow)",
    );

    // Hoisting. This code is wrong, but it's not related with unused-vars
    assert_lint_ok::<NoUnusedVars>("console.log(a); var a = 1;");

    // Ported form eslint

    assert_lint_ok::<NoUnusedVars>("var foo = 5;\n\nlabel: while (true) {\n  console.log(foo);\n  break label;\n}");
    assert_lint_ok::<NoUnusedVars>(
      "var foo = 5;\n\nwhile (true) {\n  console.log(foo);\n  break;\n}",
    );

    assert_lint_ok::<NoUnusedVars>(
      "for (let prop in box) {\n        box[prop] = parseInt(box[prop]);\n}",
    );
    assert_lint_ok::<NoUnusedVars>("var box = {a: 2};\n    for (var prop in box) {\n        box[prop] = parseInt(box[prop]);\n}");
    assert_lint_ok::<NoUnusedVars>("f({ set foo(a) { return; } });");
    assert_lint_ok::<NoUnusedVars>("a; var a;");
    assert_lint_ok::<NoUnusedVars>("var a=10; alert(a);");
    assert_lint_ok::<NoUnusedVars>("var a=10; (function() { alert(a); })();");
    assert_lint_ok::<NoUnusedVars>(
      "var a=10; (function() { setTimeout(function() { alert(a); }, 0); })();",
    );
    assert_lint_ok::<NoUnusedVars>("var a=10; d[a] = 0;");
    assert_lint_ok::<NoUnusedVars>("(function() { var a=10; return a; })();");
    assert_lint_ok::<NoUnusedVars>("(function g() {})()");
    assert_lint_ok::<NoUnusedVars>("function f(a) {alert(a);}; f();");
    assert_lint_ok::<NoUnusedVars>(
      "var c = 0; function f(a){ var b = a; return b; }; f(c);",
    );
    assert_lint_ok::<NoUnusedVars>("function a(x, y){ return y; }; a();");
    assert_lint_ok::<NoUnusedVars>("var arr1 = [1, 2]; var arr2 = [3, 4]; for (var i in arr1) { arr1[i] = 5; } for (var i in arr2) { arr2[i] = 10; }");
    assert_lint_ok::<NoUnusedVars>("var a=10;");
    assert_lint_ok::<NoUnusedVars>("var min = \"min\"; Math[min];");
    assert_lint_ok::<NoUnusedVars>("Foo.bar = function(baz) { return baz; };");
    assert_lint_ok::<NoUnusedVars>("myFunc(function foo() {}.bind(this))");
    assert_lint_ok::<NoUnusedVars>("myFunc(function foo(){}.toString())");
    assert_lint_ok::<NoUnusedVars>("function foo(first, second) {\ndoStuff(function() {\nconsole.log(second);});}; foo()");
    assert_lint_ok::<NoUnusedVars>("(function() { var doSomething = function doSomething() {}; doSomething() }())");
    assert_lint_ok::<NoUnusedVars>("try {} catch(e) {}");
    assert_lint_ok::<NoUnusedVars>("/*global a */ a;");
    assert_lint_ok::<NoUnusedVars>("var a=10; (function() { alert(a); })();");
    assert_lint_ok::<NoUnusedVars>("var a=10; (function() { alert(a); })();");
    assert_lint_ok::<NoUnusedVars>(
      "function g(bar, baz) { return bar; }; g();",
    );
    assert_lint_ok::<NoUnusedVars>("(function z() { z(); })();");
    assert_lint_ok::<NoUnusedVars>(
      "var who = \"Paul\";\nmodule.exports = `Hello ${who}!`;",
    );
    assert_lint_ok::<NoUnusedVars>("export var foo = 123;");
    assert_lint_ok::<NoUnusedVars>("export function foo () {}");
    assert_lint_ok::<NoUnusedVars>(
      "let toUpper = (partial) => partial.toUpperCase; export {toUpper}",
    );
    assert_lint_ok::<NoUnusedVars>("export class foo {}");
    assert_lint_ok::<NoUnusedVars>("class Foo{}; var x = new Foo(); x.foo()");
    assert_lint_ok::<NoUnusedVars>("const foo = \"hello!\";function bar(foobar = foo) {  foobar.replace(/!$/, \" world!\");}\nbar();");
    assert_lint_ok::<NoUnusedVars>(
      "function Foo(){}; var x = new Foo(); x.foo()",
    );
    assert_lint_ok::<NoUnusedVars>(
      "function foo() {var foo = 1; return foo}; foo();",
    );
    assert_lint_ok::<NoUnusedVars>("function foo(foo) {return foo}; foo(1);");
    assert_lint_ok::<NoUnusedVars>(
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
    );
    assert_lint_ok::<NoUnusedVars>(
      "function foo() {var foo = 1; return foo}; foo();",
    );
    assert_lint_ok::<NoUnusedVars>("function foo(foo) {return foo}; foo(1);");
    assert_lint_ok::<NoUnusedVars>(
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
    );
    assert_lint_ok::<NoUnusedVars>("const x = 1; const [y = x] = []; foo(y);");
    assert_lint_ok::<NoUnusedVars>("const x = 1; const {y = x} = {}; foo(y);");
    assert_lint_ok::<NoUnusedVars>(
      "const x = 1; const {z: [y = x]} = {}; foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "const x = []; const {z: [y] = x} = {}; foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>("const x = 1; let y; [y = x] = []; foo(y);");
    assert_lint_ok::<NoUnusedVars>(
      "const x = 1; let y; ({z: [y = x]} = {}); foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "const x = []; let y; ({z: [y] = x} = {}); foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "const x = 1; function foo(y = x) { bar(y); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>(
      "const x = 1; function foo({y = x} = {}) { bar(y); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("const x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();");
    assert_lint_ok::<NoUnusedVars>(
      "const x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("var x = 1; var [y = x] = []; foo(y);");
    assert_lint_ok::<NoUnusedVars>("var x = 1; var {y = x} = {}; foo(y);");
    assert_lint_ok::<NoUnusedVars>("var x = 1; var {z: [y = x]} = {}; foo(y);");
    assert_lint_ok::<NoUnusedVars>(
      "var x = []; var {z: [y] = x} = {}; foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>("var x = 1, y; [y = x] = []; foo(y);");
    assert_lint_ok::<NoUnusedVars>(
      "var x = 1, y; ({z: [y = x]} = {}); foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "var x = [], y; ({z: [y] = x} = {}); foo(y);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "var x = 1; function foo(y = x) { bar(y); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>(
      "var x = 1; function foo({y = x} = {}) { bar(y); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("var x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();");
    assert_lint_ok::<NoUnusedVars>(
      "var x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("var _a");
    assert_lint_ok::<NoUnusedVars>("var a; function foo() { var _b; } foo();");
    assert_lint_ok::<NoUnusedVars>("function foo(_a) { } foo();");
    assert_lint_ok::<NoUnusedVars>("function foo(a, _b) { return a; } foo();");
    assert_lint_ok::<NoUnusedVars>(
      "var [ firstItemIgnored, secondItem ] = items;\nconsole.log(secondItem);",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { var name; for ( name in obj ) return; })({});",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { var name; for ( name in obj ) { return; } })({});",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( var name in obj ) { return true } })({})",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( var name in obj ) return true })({})",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { let name; for ( name in obj ) return; })({});",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { let name; for ( name in obj ) { return; } })({});",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( let name in obj ) { return true } })({})",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( let name in obj ) return true })({})",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( const name in obj ) { return true } })({})",
    );
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { for ( const name in obj ) return true })({})",
    );
    assert_lint_ok::<NoUnusedVars>("try{}catch(err){console.error(err);}");
    assert_lint_ok::<NoUnusedVars>("try{}catch(_ignoreErr){}");
    assert_lint_ok::<NoUnusedVars>("var a = 0, b; b = a = a + 1; foo(b);");
    assert_lint_ok::<NoUnusedVars>("var a = 0, b; b = a += a + 1; foo(b);");
    assert_lint_ok::<NoUnusedVars>("var a = 0, b; b = a++; foo(b);");
    assert_lint_ok::<NoUnusedVars>(
      "function foo(a) { var b = a = a + 1; bar(b) } foo();",
    );
    assert_lint_ok::<NoUnusedVars>(
      "function foo(a) { var b = a += a + 1; bar(b) } foo();",
    );
    assert_lint_ok::<NoUnusedVars>(
      "function foo(a) { var b = a++; bar(b) } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("function foo(cb) { cb = function() { function something(a) { cb(1 + a); } register(something); }(); } foo();");
    assert_lint_ok::<NoUnusedVars>(
      "function* foo(cb) { cb = yield function(a) { cb(1 + a); }; } foo();",
    );
    assert_lint_ok::<NoUnusedVars>("function foo(cb) { cb = tag`hello${function(a) { cb(1 + a); }}`; } foo();");
    assert_lint_ok::<NoUnusedVars>("function foo(cb) { var b; cb = b = function(a) { cb(1 + a); }; b(); } foo();");
    assert_lint_ok::<NoUnusedVars>("(class { set foo(UNUSED) {} })");
    assert_lint_ok::<NoUnusedVars>(
      "class Foo { set bar(UNUSED) {} } console.log(Foo)",
    );
    assert_lint_ok::<NoUnusedVars>("var a = function () { a(); }; a();");
    assert_lint_ok::<NoUnusedVars>(
      "var a = function(){ return function () { a(); } }; a();",
    );
    assert_lint_ok::<NoUnusedVars>("const a = () => { a(); }; a();");
    assert_lint_ok::<NoUnusedVars>("const a = () => () => { a(); }; a();");
    assert_lint_ok::<NoUnusedVars>(r#"export * as ns from "source""#);
    assert_lint_ok::<NoUnusedVars>("import.meta");
  }

  #[test]
  fn no_unused_vars_err() {
    assert_lint_err::<NoUnusedVars>("var a = 0", 4);

    // variable shadowing
    assert_lint_err::<NoUnusedVars>(
      "var a = 1; function foo() { var a = 2; console.log(a); }",
      4,
    );
  }
}
