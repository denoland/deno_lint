// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::{
  ast::{
    ArrowExpr, CatchClause, ClassMethod, Constructor, Decl, ExportDecl,
    ExportNamedSpecifier, Expr, FnDecl, FnExpr, Ident, ImportDefaultSpecifier,
    ImportNamedSpecifier, ImportStarAsSpecifier, MemberExpr, MethodKind,
    Module, NamedExport, Param, Pat, SetterProp, TsEntityName,
    TsExprWithTypeArgs, TsTypeRef, VarDeclOrPat, VarDeclarator,
  },
  visit::VisitWith,
};

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

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut collector = Collector {
      used_vars: Default::default(),
      cur_defining: Default::default(),
      used_types: Default::default(),
    };
    module.visit_with(module, &mut collector);

    dbg!(&collector.used_vars, &collector.used_types);

    let mut visitor = NoUnusedVarVisitor::new(
      context,
      collector.used_vars,
      collector.used_types,
    );
    module.visit_with(module, &mut visitor);
  }
}

/// Collects information about variable usages.
struct Collector {
  used_vars: HashSet<Id>,
  used_types: HashSet<Id>,
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
  fn visit_ts_type_ref(&mut self, ty: &TsTypeRef, _: &dyn Node) {
    ty.type_params.visit_with(ty, self);

    let id = get_id(&ty.type_name);
    self.used_types.insert(id);
  }

  fn visit_ts_expr_with_type_args(
    &mut self,
    n: &TsExprWithTypeArgs,
    _: &dyn Node,
  ) {
    let id = get_id(&n.expr);
    self.used_vars.insert(id);
    n.type_args.visit_with(n, self);
  }

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
      Pat::Ident(i) => {
        i.type_ann.visit_with(pat, self);
      }
      Pat::Invalid(..) => {}
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

  fn visit_constructor(&mut self, c: &Constructor, _: &dyn Node) {
    if c.body.is_none() {
      return;
    }

    c.visit_children_with(self);
  }
}

fn get_id(r: &TsEntityName) -> Id {
  match r {
    TsEntityName::TsQualifiedName(q) => get_id(&q.left),
    TsEntityName::Ident(i) => i.to_id(),
  }
}

struct NoUnusedVarVisitor {
  context: Arc<Context>,
  used_vars: HashSet<Id>,
  used_types: HashSet<Id>,
}

impl NoUnusedVarVisitor {
  fn new(
    context: Arc<Context>,
    used_vars: HashSet<Id>,
    used_types: HashSet<Id>,
  ) -> Self {
    Self {
      context,
      used_vars,
      used_types,
    }
  }
}

impl NoUnusedVarVisitor {
  // fn handle_type_id(&mut self, ident: &Ident) {
  //   if ident.sym.starts_with('_') {
  //     return;
  //   }
  //   if !self.used_types.contains(&ident.to_id()) {
  //     // The variable is not used.
  //     self.context.add_diagnostic(
  //       ident.span,
  //       "no-unused-vars",
  //       &format!("\"{}\" is never used", ident.sym),
  //     );
  //   }
  // }

  fn handle_id(&mut self, ident: &Ident) {
    if ident.sym.starts_with('_') {
      return;
    }

    if !self.used_vars.contains(&ident.to_id()) {
      // The variable is not used.
      self.context.add_diagnostic(
        ident.span,
        "no-unused-vars",
        &format!("\"{}\" is never used", ident.sym),
      );
    }
  }
}

impl Visit for NoUnusedVarVisitor {
  fn visit_arrow_expr(&mut self, expr: &ArrowExpr, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&expr.params);

    for ident in declared_idents {
      self.handle_id(&ident);
    }
    expr.body.visit_with(expr, self)
  }

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

  fn visit_catch_clause(&mut self, clause: &CatchClause, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&clause.param);

    for ident in declared_idents {
      self.handle_id(&ident);
    }

    clause.body.visit_with(clause, self);
  }

  fn visit_setter_prop(&mut self, prop: &SetterProp, _: &dyn Node) {
    prop.key.visit_with(prop, self);
    prop.body.visit_with(prop, self);
  }

  fn visit_class_method(&mut self, method: &ClassMethod, _: &dyn Node) {
    method.function.decorators.visit_with(method, self);
    method.key.visit_with(method, self);

    match method.kind {
      MethodKind::Method => method.function.params.visit_children_with(self),
      MethodKind::Getter => {}
      MethodKind::Setter => {}
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

  fn visit_import_named_specifier(
    &mut self,
    import: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }
    self.handle_id(&import.local);
  }

  fn visit_import_default_specifier(
    &mut self,
    import: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }
    self.handle_id(&import.local);
  }

  fn visit_import_star_as_specifier(
    &mut self,
    import: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }
    self.handle_id(&import.local);
  }

  /// no-op as export is kind of usage
  fn visit_export_decl(&mut self, export: &ExportDecl, _: &dyn Node) {
    match &export.decl {
      Decl::Class(c) => {
        c.class.visit_with(c, self);
      }
      Decl::Fn(f) => {
        f.function.visit_with(f, self);
      }
      Decl::Var(v) => {
        for decl in &v.decls {
          decl.name.visit_with(decl, self);
          decl.init.visit_with(decl, self);
        }
      }
      _ => {}
    }
  }

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
    assert_lint_err::<NoUnusedVars>("var min = {min: 1}", 4);
    assert_lint_err::<NoUnusedVars>(
      "function gg(baz, bar) { return baz; }; gg();",
      17,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function(foo, baz, bar) { return baz; })();",
      vec![10, 20],
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function z(foo) { var bar = 33; })();",
      vec![12, 23],
    );
    assert_lint_err::<NoUnusedVars>("(function z(foo) { z(); })();", 12);
    assert_lint_err_n::<NoUnusedVars>(
      "function f() { var a = 1; return function(){ f(a = 2); }; }",
      vec![9, 19],
    );
    assert_lint_err::<NoUnusedVars>("import x from \"y\";", 7);
    assert_lint_err::<NoUnusedVars>(
      "export function fn2({ x, y }) {\n console.log(x); \n};",
      25,
    );
    assert_lint_err::<NoUnusedVars>(
      "export function fn2( x, y ) {\n console.log(x); \n};",
      24,
    );
  }

  #[test]
  fn no_unused_vars_err_5() {
    assert_lint_err::<NoUnusedVars>("var _a; var b;", 12);
    assert_lint_err::<NoUnusedVars>("function foo(a, _b) { } foo()", 13);
    assert_lint_err::<NoUnusedVars>(
      "function foo(a, _b, c) { return a; } foo();",
      20,
    );
  }

  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_for_loop_control_flow() {
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
  }

  #[test]
  fn no_unused_vars_err_destructuring() {
    assert_lint_err::<NoUnusedVars>(
      "const data = { type: 'coords', x: 1, y: 2 };\
     const { type, ...coords } = data;\n console.log(coords);",
      52,
    );
    assert_lint_err::<NoUnusedVars>(
      "const data = { type: 'coords', x: 3, y: 2 };\
        const { type, ...coords } = data;\n console.log(type)",
      61,
    );
    assert_lint_err::<NoUnusedVars>(
      "const data = { vars: \
      ['x','y'], x: 1, y: 2 }; const { vars: [x], ...coords } = data;\n\
       console.log(coords)",
      61,
    );
  }

  #[test]
  fn no_unused_vars_err_6() {
    assert_lint_err::<NoUnusedVars>("const data = { defaults: { x: 0 }, x: 1, y: 2 }; const { defaults: { x }, ...coords } = data;\n console.log(coords)", 69);
    assert_lint_err::<NoUnusedVars>("export default function(a) {}", 24);
    assert_lint_err::<NoUnusedVars>(
      "export default function(a, b) { console.log(a); }",
      27,
    );
    assert_lint_err::<NoUnusedVars>("export default (function(a) {});", 25);
    assert_lint_err::<NoUnusedVars>(
      "export default (function(a, b) { console.log(a); });",
      28,
    );
    assert_lint_err::<NoUnusedVars>("export default (a) => {};", 16);
    assert_lint_err::<NoUnusedVars>(
      "export default (a, b) => { console.log(a); };",
      19,
    );
    assert_lint_err::<NoUnusedVars>("try{}catch(err){};", 11);
  }

  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_expr() {
    assert_lint_err::<NoUnusedVars>("var a = 0; a = a + 1;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a = a + a;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a += a + 1", 0);
    assert_lint_err::<NoUnusedVars>("var a = 0; a++;", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a = a + 1 } foo();", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a += a + 1 } foo();", 0);
    assert_lint_err::<NoUnusedVars>("function foo(a) { a++ } foo();", 0);
    assert_lint_err::<NoUnusedVars>("var a = 3; a = a * 5 + 6;", 0);
    assert_lint_err::<NoUnusedVars>("var a = 2, b = 4; a = a * 2 + b;", 0);

    assert_lint_err::<NoUnusedVars>("const a = 1; a += 1;", 6);
  }

  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_to_self() {
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
  }

  #[test]
  fn no_unused_vars_err_10() {
    assert_lint_err::<NoUnusedVars>(
      "(function ({ a }, b ) { return b; })();",
      13,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ({ a }, { b, c } ) { return b; })();",
      vec![13, 23],
    );
    assert_lint_err::<NoUnusedVars>(
      "(function ([ a ], b ) { return b; })();",
      13,
    );
    assert_lint_err_n::<NoUnusedVars>(
      "(function ([ a ], [ b, c ] ) { return b; })();",
      vec![13, 23],
    );
  }

  #[test]
  fn no_unused_vars_err_11() {
    assert_lint_err::<NoUnusedVars>("var a = function() { a(); };", 4);
    assert_lint_err::<NoUnusedVars>(
      "var a = function(){ return function() { a(); } };",
      4,
    );
    assert_lint_err::<NoUnusedVars>("const a = () => { a(); };", 6);
    assert_lint_err::<NoUnusedVars>("const a = () => () => { a(); };", 6);

    assert_lint_err::<NoUnusedVars>("var a = function() { a(); };", 4);
    assert_lint_err::<NoUnusedVars>(
      "var a = function(){ return function() { a(); } };",
      4,
    );
    assert_lint_err::<NoUnusedVars>("const a = () => { a(); };", 6);
    assert_lint_err::<NoUnusedVars>("const a = () => () => { a(); };", 6);
  }

  #[test]
  #[ignore = "pure method analysis is not implemented yet"]
  fn no_unused_vars_err_array_methods() {
    assert_lint_err::<NoUnusedVars>(
      "let myArray = [1,2,3,4].filter((x) => x == 0); myArray = myArray.filter((x) => x == 1);",
      4,
    );
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

  #[test]
  fn no_unused_vars_ts_ok_1() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { ClassDecoratorFactory } from 'decorators';
@ClassDecoratorFactory()
export class Foo {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { ClassDecorator } from 'decorators';
@ClassDecorator
export class Foo {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { AccessorDecoratorFactory } from 'decorators';
export class Foo {
  @AccessorDecoratorFactory(true)
  get bar() {}
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { AccessorDecorator } from 'decorators';
export class Foo {
  @AccessorDecorator
  set bar() {}
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { MethodDecoratorFactory } from 'decorators';
export class Foo {
  @MethodDecoratorFactory(false)
  bar() {}
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_2() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { MethodDecorator } from 'decorators';
export class Foo {
  @MethodDecorator
  static bar() {}
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { ConstructorParameterDecoratorFactory } from 'decorators';
export class Service {
  constructor(
    @ConstructorParameterDecoratorFactory(APP_CONFIG) config: AppConfig,
  ) {
    this.title = config.title;
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { ConstructorParameterDecorator } from 'decorators';
export class Foo {
  constructor(@ConstructorParameterDecorator bar) {
    this.bar = bar;
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { ParameterDecoratorFactory } from 'decorators';
export class Qux {
  bar(@ParameterDecoratorFactory(true) baz: number) {
    console.log(baz);
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { ParameterDecorator } from 'decorators';
export class Foo {
  static greet(@ParameterDecorator name: string) {
    return name;
  }
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_3() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Input, Output, EventEmitter } from 'decorators';
export class SomeComponent {
  @Input() data;
  @Output()
  click = new EventEmitter();
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { configurable } from 'decorators';
export class A {
  @configurable(true) static prop1;
  @configurable(false)
  static prop2;
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { foo, bar } from 'decorators';
export class B {
  @foo x;
  @bar
  y;
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
interface Base {}
class Thing implements Base {}
new Thing();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
interface Base {}
const a: Base = {};
console.log(a);
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_4() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from 'foo';
function bar<T>() {}
bar<Foo>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from 'foo';
const bar = function <T>() {};
bar<Foo>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from 'foo';
const bar = <T>() => {};
bar<Foo>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from 'foo';
<Foo>(<T>() => {})();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Nullable<string> = 'hello';
console.log(a);
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_5() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther> = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Nullable | undefined = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Nullable & undefined = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther[]> = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<Array<SomeOther>> = 'hello';
console.log(a);
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_6() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Array<Nullable> = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Nullable[] = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Array<Nullable[]> = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
const a: Array<Array<Nullable>> = 'hello';
console.log(a);
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Array<Nullable<SomeOther>> = 'hello';
console.log(a);
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_7() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo implements Component<Nullable> {}
new Foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo extends Component<Nullable, {}> {}
new Foo();
          ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo extends Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component, Component2 } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}>, Component2 {}
new Foo();
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_8() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do = (a: Nullable<Another>) => {
    console.log(a);
  };
}
new A();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(a: Nullable<Another>) {
    console.log(a);
  }
}
new A();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(): Nullable<Another> {
    return null;
  }
}
new A();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
function foo(a: Nullable) {
  console.log(a);
}
foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_9() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
class A extends Nullable<SomeOther> {
  do(a: Nullable<Another>) {
    console.log(a);
  }
}
new A();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  other: Nullable<Another>;
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  do(a: Nullable<Another>);
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
class Bar<T extends Foo> {}
new Bar<number>();
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_10() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo, Bar } from './types';
class Baz<T extends Foo & Bar> {}
new Baz<any>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
class Bar<T = Foo> {}
new Bar<number>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
class Foo<T = any> {}
new Foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
class Foo<T = {}> {}
new Foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
class Foo<T extends {} = {}> {}
new Foo();
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_11() {
    assert_lint_ok::<NoUnusedVars>(
      "
type Foo = 'a' | 'b' | 'c';
type Bar = number;
export const map: { [name in Foo]: Bar } = {
  a: 1,
  b: 2,
  c: 3,
};
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
class A<T> {
  bar: T;
}
new A<Nullable>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
function foo<T extends Nullable>() {}
foo<SomeOther>();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
class A<T extends Nullable> {
  bar: T;
}
new A<SomeOther>()
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
interface A<T extends Nullable> {
  bar: T;
}
export const a: A<SomeOther> = {
  foo: 'bar',
};

      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_12() {
    assert_lint_ok::<NoUnusedVars>(
      "
export class App {
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
export class App {
  constructor(bar: string);
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
export class App {
  constructor(baz: string, private logger: Logger) {
    console.log(baz);
    console.log(this.logger);
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
export class App {
  constructor(baz: string, private logger: Logger, private bar: () => void) {
    console.log(this.logger);
    this.bar();
  }
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
export class App {
  constructor(private logger: Logger) {}
  meth() {
    console.log(this.logger);
  }
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_13() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Component, Vue } from 'vue-property-decorator';
import HelloWorld from './components/HelloWorld.vue';
@Component({
  components: {
    HelloWorld,
  },
})
export default class App extends Vue {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import firebase, { User } from 'firebase/app';
// initialize firebase project
firebase.initializeApp({});
export function authenticated(cb: (user: User | null) => void): void {
  firebase.auth().onAuthStateChanged(user => cb(user));
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo } from './types';
export class Bar<T extends Foo> {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import webpack from 'webpack';
export default function webpackLoader(this: webpack.loader.LoaderContext) {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import execa, { Options as ExecaOptions } from 'execa';
export function foo(options: ExecaOptions): execa {
  options();
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_14() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Foo, Bar } from './types';
export class Baz<F = Foo & Bar> {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
// warning 'B' is defined but never used
export const a: Array<{ b: B }> = [];
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
export enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds;
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds.EMAIL;
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_15() {
    assert_lint_ok::<NoUnusedVars>(
      "
import * as fastify from 'fastify';
import { Server, IncomingMessage, ServerResponse } from 'http';
const server: fastify.FastifyInstance<
  Server,
  IncomingMessage,
  ServerResponse
> = fastify({});
server.get('/ping');
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
declare function foo();
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
declare namespace Foo {
  function bar(line: string, index: number | null, tabSize: number): number;
  var baz: string;
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
declare var Foo: {
  new (value?: any): Object;
  foo(): string;
};
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
declare class Foo {
  constructor(value?: any): Object;
  foo(): string;
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_16() {
    assert_lint_ok::<NoUnusedVars>(
      "
import foo from 'foo';
export interface Bar extends foo.i18n {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import foo from 'foo';
import bar from 'foo';
export interface Bar extends foo.i18n<bar> {}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { TypeA } from './interface';
export const a = <GenericComponent<TypeA> />;
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
const text = 'text';
export function Foo() {
  return (
    <div>
      <input type=\"search\" size={30} placeholder={text} />
    </div>
  );
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { observable } from 'mobx';
export default class ListModalStore {
  @observable
  orderList: IObservableArray<BizPurchaseOrderTO> = observable([]);
}
      ",
    );
  }

  #[test]
  fn no_unused_vars_ts_ok_17() {
    assert_lint_ok::<NoUnusedVars>(
      "
import { Dec, TypeA, Class } from 'test';
export default class Foo {
  constructor(
    @Dec(Class)
    private readonly prop: TypeA<Class>,
  ) {}
}
      ",
    );

    assert_lint_ok::<NoUnusedVars>(
      "
import { Dec, TypeA, Class } from 'test';
export default class Foo {
  constructor(
    @Dec(Class)
    ...prop: TypeA<Class>
  ) {
    prop();
  }
}
      ",
    );
  }

  // TODO: Copy https://github.com/typescript-eslint/typescript-eslint/blob/6f397df42cbcf05c10f304c9bbfdae4803aa0ce2/packages/eslint-plugin/tests/rules/no-unused-vars.test.ts#L621
}
