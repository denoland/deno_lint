// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::ExportDecl;
use swc_ecmascript::ast::ExportNamedSpecifier;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::FnDecl;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::NamedExport;
use swc_ecmascript::ast::{
  ClassMethod, FnExpr, Param, Pat, SetterProp, VarDeclOrPat, VarDeclarator,
};
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
      cur_defining: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoUnusedVarVisitor::new(context, collector.used_vars);
    module.visit_with(module, &mut visitor);
  }
}

/// Collects information about variable usages.
struct Collector {
  used_vars: HashSet<Id>,
  /// Currently defining functions or variables.
  ///
  ///
  /// Note: As resolver handles binding-binding conflict of identifiers,
  /// we can safely remove an ident from the set after declaration.
  /// I mean, all binding identifiers are unique up to symbol and syntax context.
  ///
  ///
  /// Type of this should be hashset, but we don't have a way to
  /// restore hashset after handling bindings
  cur_defining: Vec<Id>,
}

impl Visit for Collector {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc
  // It will make binary much smaller. In case of swc, binary size is reduced to 18mb from 29mb.
  // noop_visit_type!();

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    match expr {
      Expr::Ident(i) => {
        let id = i.to_id();

        // Recursive calls are not usage
        if self.cur_defining.contains(&id) {
          return;
        }

        // Mark the variable as used.
        self.used_vars.insert(id);
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

  /// export is kind of usage
  fn visit_export_named_specifier(
    &mut self,
    export: &ExportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.used_vars.insert(export.orig.to_id());
  }

  /// Handl for-in/of loops
  fn visit_var_decl_or_pat(&mut self, node: &VarDeclOrPat, _: &dyn Node) {
    // We need this because find_ids searches ids only in the pattern.
    node.visit_children_with(self);

    match node {
      VarDeclOrPat::VarDecl(v) => {
        // This is declaration, but cannot be removed.
        let ids = find_ids(v);
        self.used_vars.extend(ids);
      }
      VarDeclOrPat::Pat(p) => {
        // This is assignment, but cannot be removed
        let ids = find_ids(p);
        self.used_vars.extend(ids);
      }
    }
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl, _: &dyn Node) {
    let id = decl.ident.to_id();
    self.cur_defining.push(id);
    decl.function.visit_with(decl, self);
    self.cur_defining.pop();
  }

  fn visit_fn_expr(&mut self, expr: &FnExpr, _: &dyn Node) {
    if let Some(ident) = &expr.ident {
      let id = ident.to_id();
      self.cur_defining.push(id);
      expr.function.visit_with(expr, self);
      self.cur_defining.pop();
    } else {
      expr.function.visit_with(expr, self);
    }
  }

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator, _: &dyn Node) {
    let prev_len = self.cur_defining.len();
    let declaring_ids: Vec<Id> = find_ids(&declarator.name);
    self.cur_defining.extend(declaring_ids);

    declarator.name.visit_with(declarator, self);
    declarator.init.visit_with(declarator, self);

    // Restore the original state
    self.cur_defining.drain(prev_len..);
    assert_eq!(self.cur_defining.len(), prev_len);
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

impl NoUnusedVarVisitor {
  fn handle_id(&mut self, ident: &Ident) {
    if ident.sym.starts_with('_') {
      return;
    }

    if !self.used_vars.contains(&ident.to_id()) {
      dbg!(&ident.to_id());
      // The variable is not used.
      self.context.add_diagnostic(
        ident.span,
        "no-unused-vars",
        &format!("\"{}\" is never used", ident.sym),
      );
    }
  }
}

/// As we only care about variables, only variable declrations are checked.
impl Visit for NoUnusedVarVisitor {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc

  fn visit_fn_decl(&mut self, decl: &FnDecl, _: &dyn Node) {
    self.handle_id(&decl.ident);
    decl.function.visit_with(decl, self);
  }

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&declarator.name);

    for ident in declared_idents {
      self.handle_id(&ident);
    }
    declarator.name.visit_with(declarator, self);
    declarator.init.visit_with(declarator, self);
  }

  fn visit_setter_prop(&mut self, prop: &SetterProp, _: &dyn Node) {
    prop.key.visit_with(prop, self);
    prop.body.visit_with(prop, self);
  }

  fn visit_class_method(&mut self, method: &ClassMethod, _: &dyn Node) {
    method.function.decorators.visit_with(method, self);
    method.key.visit_with(method, self);

    match method.kind {
      swc_ecmascript::ast::MethodKind::Method => {
        method.function.params.visit_children_with(self)
      }
      swc_ecmascript::ast::MethodKind::Getter => {}
      swc_ecmascript::ast::MethodKind::Setter => {}
    }

    method.function.body.visit_with(method, self);
  }

  fn visit_param(&mut self, param: &Param, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&param.pat);

    for ident in declared_idents {
      self.handle_id(&ident);
    }
    param.visit_children_with(self)
  }

  /// no-op as export is kind of usage
  fn visit_export_decl(&mut self, _: &ExportDecl, _: &dyn Node) {}

  /// no-op as export is kind of usage
  fn visit_named_export(&mut self, _: &NamedExport, _: &dyn Node) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unused_vars_ok_1() {
    assert_lint_ok::<NoUnusedVars>("var a = 1; console.log(a)");
    assert_lint_ok::<NoUnusedVars>(
      "var a = 1; const arrow = () => a; console.log(arrow)",
    );

    // Hoisting. This code is wrong, but it's not related with unused-vars
    assert_lint_ok::<NoUnusedVars>("console.log(a); var a = 1;");
  }
  #[test]
  fn no_unused_vars_ok_2() {
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
  }

  #[test]
  fn no_unused_vars_ok_3() {
    assert_lint_ok::<NoUnusedVars>("var arr1 = [1, 2]; var arr2 = [3, 4]; for (var i in arr1) { arr1[i] = 5; } for (var i in arr2) { arr2[i] = 10; }");
    assert_lint_ok::<NoUnusedVars>("var min = \"min\"; Math[min];");
    assert_lint_ok::<NoUnusedVars>("Foo.bar = function(baz) { return baz; };");
    assert_lint_ok::<NoUnusedVars>("myFunc(function foo() {}.bind(this))");
    assert_lint_ok::<NoUnusedVars>("myFunc(function foo(){}.toString())");
    assert_lint_ok::<NoUnusedVars>("(function() { var doSomething = function doSomething() {}; doSomething() }())");
    assert_lint_ok::<NoUnusedVars>("try {} catch(e) {}");
    assert_lint_ok::<NoUnusedVars>("/*global a */ a;");
    assert_lint_ok::<NoUnusedVars>("var a=10; (function() { alert(a); })();");
    assert_lint_ok::<NoUnusedVars>("var a=10; (function() { alert(a); })();");
  }

  #[test]
  fn no_unused_vars_ok_4() {
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
  }

  #[test]
  fn no_unused_vars_ok_5() {
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
  }

  #[test]
  fn no_unused_vars_ok_6() {
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
    assert_lint_ok::<NoUnusedVars>("function foo(_a) { } foo();");
    assert_lint_ok::<NoUnusedVars>("function foo(a, _b) { return a; } foo();");
    assert_lint_ok::<NoUnusedVars>(
      "(function(obj) { var name; for ( name in obj ) return; })({});",
    );
  }

  #[test]
  fn no_unused_vars_ok_7() {
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
  }

  #[test]
  fn no_unused_vars_ok_8() {
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
  }

  #[test]
  fn no_unused_vars_ok_9() {
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
  }

  #[test]
  fn no_unused_vars_ok_10() {
    assert_lint_ok::<NoUnusedVars>("const a = () => { a(); }; a();");
    assert_lint_ok::<NoUnusedVars>("const a = () => () => { a(); }; a();");
    assert_lint_ok::<NoUnusedVars>(r#"export * as ns from "source""#);
    assert_lint_ok::<NoUnusedVars>("import.meta");
  }

  #[test]
  fn no_unused_vars_err_1() {
    assert_lint_err::<NoUnusedVars>("var a = 0", 4);

    // variable shadowing
    assert_lint_err::<NoUnusedVars>(
      "var a = 1; function foo() { var a = 2; console.log(a); }; use(foo);",
      4,
    );
  }

  #[test]
  fn no_unused_vars_err_2() {
    assert_lint_err::<NoUnusedVars>("function foox() { return foox(); }", 9);
    assert_lint_err::<NoUnusedVars>(
      "(function() { function foox() { if (true) { return foox(); } } }())",
      23,
    );

    assert_lint_err::<NoUnusedVars>("var a=10", 4);
    assert_lint_err::<NoUnusedVars>(
      "function f() { var a = 1; return function(){ f(a *= 2); }; }",
      9,
    );

    assert_lint_err::<NoUnusedVars>(
      "function f() { var a = 1; return function(){ f(++a); }; }",
      9,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "function foo(first, second) {\ndoStuff(function()\
       {\nconsole.log(second);});};",
      vec![9, 13],
    );

    assert_lint_err::<NoUnusedVars>("var a=10;", 4);
    assert_lint_err::<NoUnusedVars>("var a=10; a=20;", 4);

    assert_lint_err_n::<NoUnusedVars>(
      "var a=10; (function() { var a = 1; alert(a); })();",
      vec![4],
    );
    assert_lint_err::<NoUnusedVars>("var a=10, b=0, c=null; alert(a+b)", 15);
  }

  #[test]
  fn no_unused_vars_err_3() {
    assert_lint_err::<NoUnusedVars>("var a=10, b=0, c=null; setTimeout(function() { var b=2; alert(a+b+c); }, 0);", 10);
    assert_lint_err_n::<NoUnusedVars>(
      "var a=10, b=0, c=null; setTimeout(function() \
      { var b=2; var c=2; alert(a+b+c); }, 0);",
      vec![10, 15],
    );
    assert_lint_err::<NoUnusedVars>(
      "function f(){var a=[];return a.map(function(){});}",
      9,
    );
    assert_lint_err::<NoUnusedVars>(
      "function f(){var a=[];return a.map(function g(){});}",
      9,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "function f(){var x;function a(){x=42;}function b(){alert(x);}}",
      vec![9, 28, 47],
    );
    assert_lint_err::<NoUnusedVars>("function f(a) {}; f();", 11);
    assert_lint_err_n::<NoUnusedVars>(
      "function a(x, y, z){ return y; }; a();",
      vec![11, 17],
    );
    assert_lint_err::<NoUnusedVars>("var min = Math.min", 4);
    assert_lint_err::<NoUnusedVars>("var min = {min: 1}", 4);
    assert_lint_err::<NoUnusedVars>(
      "Foo.bar = function(baz) { return 1; };",
      19,
    );
  }

  #[test]
  fn no_unused_vars_err_4() {
    assert_lint_err::<NoUnusedVars>("var min = {min: 1}", 0);
    assert_lint_err::<NoUnusedVars>(
      "function gg(baz, bar) { return baz; }; gg();",
      0,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function(foo, baz, bar) { return baz; })();",
      vec![0, 1],
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function z(foo) { var bar = 33; })();",
      vec![0, 1],
    );
    assert_lint_err::<NoUnusedVars>("(function z(foo) { z(); })();", 0);
    assert_lint_err_n::<NoUnusedVars>(
      "function f() { var a = 1; return function(){ f(a = 2); }; }",
      vec![0, 1],
    );
    assert_lint_err::<NoUnusedVars>("import x from \"y\";", 0);
    assert_lint_err::<NoUnusedVars>(
      "export function fn2({ x, y }) {\n console.log(x); \n};",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "export function fn2( x, y ) {\n console.log(x); \n};",
      0,
    );
  }

  #[test]
  fn no_unused_vars_err_5() {
    assert_lint_err::<NoUnusedVars>("var _a; var b;", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a, _b) { } foo()", 0);
    assert_lint_err::<NoUnusedVars>(
      "function foo(a, _b, c) { return a; } foo();",
      0,
    );
    assert_lint_err::<NoUnusedVars>("function foo(_a) { } foo();", 0);
    assert_lint_err::<NoUnusedVars>(
      "(function(obj) { var name; for ( name in obj ) { i(); return; } })({});",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "(function(obj) { var name; for ( name in obj ) { } })({});",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "(function(obj) { for ( var name in obj ) { } })({});",
      0,
    );
    assert_lint_err::<NoUnusedVars>("const data = { type: 'coords', x: 1, y: 2 }; const { type, ...coords } = data;\n console.log(coords);", 0);
    assert_lint_err::<NoUnusedVars>("const data = { type: 'coords', x: 3, y: 2 }; const { type, ...coords } = data;\n console.log(type)", 0);
    assert_lint_err::<NoUnusedVars>("const data = { vars: ['x','y'], x: 1, y: 2 }; const { vars: [x], ...coords } = data;\n console.log(coords)", 0);
  }

  #[test]
  fn no_unused_vars_err_6() {
    assert_lint_err::<NoUnusedVars>("const data = { defaults: { x: 0 }, x: 1, y: 2 }; const { defaults: { x }, ...coords } = data;\n console.log(coords)", 0);
    assert_lint_err::<NoUnusedVars>("export default function(a) {}", 0);
    assert_lint_err::<NoUnusedVars>(
      "export default function(a, b) { console.log(a); }",
      0,
    );
    assert_lint_err::<NoUnusedVars>("export default (function(a) {});", 0);
    assert_lint_err::<NoUnusedVars>(
      "export default (function(a, b) { console.log(a); });",
      0,
    );
    assert_lint_err::<NoUnusedVars>("export default (a) => {};", 0);
    assert_lint_err::<NoUnusedVars>(
      "export default (a, b) => { console.log(a); };",
      0,
    );
    assert_lint_err::<NoUnusedVars>("try{}catch(err){};", 0);
  }

  #[test]
  fn no_unused_vars_err_7() {
    assert_lint_err::<NoUnusedVars>("var a = 0; a = a + 1;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a = a + a;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a += a + 1", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a++;", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a = a + 1 } foo();", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a += a + 1 } foo();", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a++ } foo();", 0);
    assert_lint_err::<NoUnusedVars>("var a = 3; a = a * 5 + 6;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 2, b = 4; a = a * 2 + b;", 0);
  }
  #[test]
  fn no_unused_vars_err_9() {
    assert_lint_err::<NoUnusedVars>("function foo(cb) { cb = function(a) { cb(1 + a); }; bar(not_cb); } foo();", 0);
    assert_lint_err::<NoUnusedVars>(
      "function foo(cb) { cb = function(a) { return cb(1 + a); }(); } foo();",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "function foo(cb) { cb = (function(a) { cb(1 + a); }, cb); } foo();",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "function foo(cb) { cb = (0, function(a) { cb(1 + a); }); } foo();",
      0,
    );
    assert_lint_err::<NoUnusedVars>(
      "(function ({ a }, b ) { return b; })();",
      0,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ({ a }, { b, c } ) { return b; })();",
      vec![0, 1],
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ({ a, b }, { c } ) { return b; })();",
      vec![0, 1],
    );
    assert_lint_err::<NoUnusedVars>(
      "(function ([ a ], b ) { return b; })();",
      0,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ([ a ], [ b, c ] ) { return b; })();",
      vec![0, 1],
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ([ a, b ], [ c ] ) { return b; })();",
      vec![0, 1],
    );
  }

  #[test]
  fn no_unused_vars_err_10() {
    assert_lint_err::<NoUnusedVars>("var a = function() { a(); };", 4);
    assert_lint_err::<NoUnusedVars>(
      "var a = function(){ return function() { a(); } };",
      4,
    );
    assert_lint_err::<NoUnusedVars>("const a = () => { a(); };", 6);
    assert_lint_err::<NoUnusedVars>("const a = () => () => { a(); };", 6);
    assert_lint_err::<NoUnusedVars>(
      "let myArray = [1,2,3,4].filter((x) => x == 0); myArray = myArray.filter((x) => x == 1);",
      4,
    );
    assert_lint_err::<NoUnusedVars>("const a = 1; a += 1;", 6);
    assert_lint_err::<NoUnusedVars>("var a = function() { a(); };", 4);
    assert_lint_err::<NoUnusedVars>(
      "var a = function(){ return function() { a(); } };",
      4,
    );
    assert_lint_err::<NoUnusedVars>("const a = () => { a(); };", 6);
    assert_lint_err::<NoUnusedVars>("const a = () => () => { a(); };", 6);
  }

  #[test]
  fn no_unused_vars_err_8() {
    assert_lint_err_on_line_n::<NoUnusedVars>(
      "let a = 'a';
    a = 10;
    function foo(){
        a = 11;
        a = () => {
            a = 13
        }
    }",
      vec![(1, 4), (3, 13)],
    );
    assert_lint_err_on_line_n::<NoUnusedVars>(
      "let c = 'c'
    c = 10
    function foo1() {
      c = 11
      c = () => {
        c = 13
      }
    }
    c = foo1",
      vec![(1, 4)],
    );
  }
}
