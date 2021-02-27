// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use std::collections::HashSet;
use swc_ecmascript::ast::{
  ArrowExpr, CatchClause, ClassDecl, ClassMethod, ClassProp, Constructor, Decl,
  ExportDecl, ExportNamedSpecifier, Expr, FnDecl, FnExpr, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  MemberExpr, MethodKind, NamedExport, Param, Pat, Program, Prop, PropName,
  SetterProp, TsEntityName, TsEnumDecl, TsExprWithTypeArgs, TsModuleDecl,
  TsNamespaceDecl, TsPropertySignature, TsTypeRef, VarDecl, VarDeclOrPat,
  VarDeclarator,
};
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::{find_ids, Id};
use swc_ecmascript::visit::{Node, Visit, VisitWith};

pub struct NoUnusedVars;

const CODE: &str = "no-unused-vars";

#[derive(Display)]
enum NoUnusedVarsMessage {
  #[display(fmt = "`{}` is never used", _0)]
  NeverUsed(String),
}

impl LintRule for NoUnusedVars {
  fn new() -> Box<Self> {
    Box::new(NoUnusedVars)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut collector = Collector::default();
    program.visit_with(program, &mut collector);

    let mut visitor = NoUnusedVarVisitor::new(
      context,
      collector.used_vars,
      collector.used_types,
    );
    program.visit_with(program, &mut visitor);
  }
}

/// Collects information about variable usages.
#[derive(Default)]
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

impl Collector {
  fn mark_as_usage(&mut self, i: &Ident) {
    let id = i.to_id();

    // Recursive calls are not usage
    if self.cur_defining.contains(&id) {
      return;
    }

    // Mark the variable as used.
    self.used_vars.insert(id);
  }
}

impl Visit for Collector {
  fn visit_class_prop(&mut self, n: &ClassProp, _: &dyn Node) {
    n.decorators.visit_with(n, self);

    if n.computed {
      n.key.visit_with(n, self);
    }

    n.value.visit_with(n, self);
    n.type_ann.visit_with(n, self);
  }

  fn visit_ts_property_signature(
    &mut self,
    n: &TsPropertySignature,
    _: &dyn Node,
  ) {
    if n.computed {
      n.key.visit_with(n, self);
    }

    n.type_params.visit_with(n, self);
    n.type_ann.visit_with(n, self);
    n.params.visit_with(n, self);
    n.init.visit_with(n, self);
  }

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

  fn visit_prop(&mut self, n: &Prop, _: &dyn Node) {
    match n {
      Prop::Shorthand(i) => self.mark_as_usage(i),
      _ => n.visit_children_with(self),
    }
  }

  fn visit_prop_name(&mut self, n: &PropName, _: &dyn Node) {
    if let PropName::Computed(computed) = n {
      computed.visit_children_with(self);
    }
    // Don't check Ident, Str, Num and BigInt
  }

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    match expr {
      Expr::Ident(i) => self.mark_as_usage(i),
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

struct NoUnusedVarVisitor<'c> {
  context: &'c mut Context,
  used_vars: HashSet<Id>,
  used_types: HashSet<Id>,
}

impl<'c> NoUnusedVarVisitor<'c> {
  fn new(
    context: &'c mut Context,
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

impl<'c> NoUnusedVarVisitor<'c> {
  fn handle_id(&mut self, ident: &Ident) {
    if ident.sym.starts_with('_') {
      return;
    }

    if !self.used_vars.contains(&ident.to_id()) {
      // The variable is not used.
      self.context.add_diagnostic(
        ident.span,
        CODE,
        NoUnusedVarsMessage::NeverUsed(ident.sym.to_string()),
      );
    }
  }
}

impl<'c> Visit for NoUnusedVarVisitor<'c> {
  fn visit_arrow_expr(&mut self, expr: &ArrowExpr, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&expr.params);

    for ident in declared_idents {
      self.handle_id(&ident);
    }
    expr.body.visit_with(expr, self)
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl, _: &dyn Node) {
    if decl.declare {
      return;
    }

    self.handle_id(&decl.ident);

    // If function body is not present, it's an overload definition
    if decl.function.body.is_some() {
      decl.function.visit_with(decl, self);
    }
  }

  fn visit_var_decl(&mut self, n: &VarDecl, _: &dyn Node) {
    if n.declare {
      return;
    }

    n.decls.visit_with(n, self);
  }

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&declarator.name);

    for ident in declared_idents {
      self.handle_id(&ident);
    }
    declarator.name.visit_with(declarator, self);
    declarator.init.visit_with(declarator, self);
  }

  fn visit_class_decl(&mut self, n: &ClassDecl, _: &dyn Node) {
    if n.declare {
      return;
    }

    n.visit_children_with(self);
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

    // If method body is not present, it's an overload definition
    if matches!(method.kind, MethodKind::Method if method.function.body.is_some())
    {
      method.function.params.visit_children_with(self)
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

  /// No error as export is kind of usage
  fn visit_export_decl(&mut self, export: &ExportDecl, _: &dyn Node) {
    match &export.decl {
      Decl::Class(c) => {
        c.class.visit_with(c, self);
      }
      Decl::Fn(f) => {
        // If function body is not present, it's an overload definition
        if f.function.body.is_some() {
          f.function.visit_with(f, self);
        }
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

  fn visit_params(&mut self, params: &[Param], parent: &dyn Node) {
    match params.first() {
      Some(Param {
        pat: Pat::Ident(i), ..
      }) if i.id.sym == *"this" => params
        .iter()
        .skip(1)
        .for_each(|param| param.visit_with(parent, self)),
      _ => params
        .iter()
        .for_each(|param| param.visit_with(parent, self)),
    }
  }

  fn visit_ts_enum_decl(&mut self, n: &TsEnumDecl, _: &dyn Node) {
    if n.declare {
      return;
    }

    if self.used_types.contains(&n.id.to_id()) {
      return;
    }
    self.handle_id(&n.id);
  }

  fn visit_ts_module_decl(&mut self, n: &TsModuleDecl, _: &dyn Node) {
    if n.declare {
      return;
    }

    n.body.visit_with(n, self);
  }

  fn visit_ts_namespace_decl(&mut self, n: &TsNamespaceDecl, _: &dyn Node) {
    if n.declare {
      return;
    }

    n.body.visit_with(n, self);
  }

  /// no-op as export is kind of usage
  fn visit_named_export(&mut self, _: &NamedExport, _: &dyn Node) {}
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unused_vars_valid() {
    assert_lint_ok! {
      NoUnusedVars,
      "var a = 1; console.log(a)",
      "var a = 1; const arrow = () => a; console.log(arrow)",
      // Hoisting. This code is wrong, but it's not related with unused-vars
      "console.log(a); var a = 1;",
      "var foo = 5;\n\nlabel: while (true) {\n  console.log(foo);\n  break label;\n}",
      "var foo = 5;\n\nwhile (true) {\n  console.log(foo);\n  break;\n}",
      "for (let prop in box) {\n        box[prop] = parseInt(box[prop]);\n}",
      "var box = {a: 2};\n    for (var prop in box) {\n        box[prop] = parseInt(box[prop]);\n}",
      "f({ set foo(a) { return; } });",
      "a; var a;",
      "var a=10; alert(a);",
      "var a=10; (function() { alert(a); })();",
      "var a=10; (function() { setTimeout(function() { alert(a); }, 0); })();",
      "var a=10; d[a] = 0;",
      "(function() { var a=10; return a; })();",
      "(function g() {})()",
      "function f(a) {alert(a);}; f();",
      "var c = 0; function f(a){ var b = a; return b; }; f(c);",
      "var arr1 = [1, 2]; var arr2 = [3, 4]; for (var i in arr1) { arr1[i] = 5; } for (var i in arr2) { arr2[i] = 10; }",
      "var min = \"min\"; Math[min];",
      "Foo.bar = function(baz) { return baz; };",
      "myFunc(function foo() {}.bind(this))",
      "myFunc(function foo(){}.toString())",
      "(function() { var doSomething = function doSomething() {}; doSomething() }())",
      "/*global a */ a;",
      "var a=10; (function() { alert(a); })();",
      "var a=10; (function() { alert(a); })();",
      "(function z() { z(); })();",
      "var who = \"Paul\";\nmodule.exports = `Hello ${who}!`;",
      "export var foo = 123;",
      "export function foo () {}",
      "let toUpper = (partial) => partial.toUpperCase; export {toUpper}",
      "export class foo {}",
      "class Foo{}; var x = new Foo(); x.foo()",
      "const foo = \"hello!\";function bar(foobar = foo) {  foobar.replace(/!$/, \" world!\");}\nbar();",
      "function Foo(){}; var x = new Foo(); x.foo()",
      "function foo() {var foo = 1; return foo}; foo();",
      "function foo(foo) {return foo}; foo(1);",
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
      "function foo() {var foo = 1; return foo}; foo();",
      "function foo(foo) {return foo}; foo(1);",
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
      "const x = 1; const [y = x] = []; foo(y);",
      "const x = 1; const {y = x} = {}; foo(y);",
      "const x = 1; const {z: [y = x]} = {}; foo(y);",
      "const x = []; const {z: [y] = x} = {}; foo(y);",
      "const x = 1; let y; [y = x] = []; foo(y);",
      "const x = 1; let y; ({z: [y = x]} = {}); foo(y);",
      "const x = []; let y; ({z: [y] = x} = {}); foo(y);",
      "const x = 1; function foo(y = x) { bar(y); } foo();",
      "const x = 1; function foo({y = x} = {}) { bar(y); } foo();",
      "const x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();",
      "const x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
      "var x = 1; var [y = x] = []; foo(y);",
      "var x = 1; var {y = x} = {}; foo(y);",
      "var x = 1; var {z: [y = x]} = {}; foo(y);",
      "var x = []; var {z: [y] = x} = {}; foo(y);",
      "var x = 1, y; [y = x] = []; foo(y);",
      "var x = 1, y; ({z: [y = x]} = {}); foo(y);",
      "var x = [], y; ({z: [y] = x} = {}); foo(y);",
      "var x = 1; function foo(y = x) { bar(y); } foo();",
      "var x = 1; function foo({y = x} = {}) { bar(y); } foo();",
      "var x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();",
      "var x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
      "var _a",
      "function foo(_a) { } foo();",
      "function foo(a, _b) { return a; } foo();",
      "(function(obj) { var name; for ( name in obj ) return; })({});",
      "(function(obj) { var name; for ( name in obj ) { return; } })({});",
      "(function(obj) { for ( var name in obj ) { return true } })({})",
      "(function(obj) { for ( var name in obj ) return true })({})",
      "(function(obj) { let name; for ( name in obj ) return; })({});",
      "(function(obj) { let name; for ( name in obj ) { return; } })({});",
      "(function(obj) { for ( let name in obj ) { return true } })({})",
      "(function(obj) { for ( let name in obj ) return true })({})",
      "(function(obj) { for ( const name in obj ) { return true } })({})",
      "(function(obj) { for ( const name in obj ) return true })({})",
      "try{}catch(err){console.error(err);}",
      "try{}catch(_ignoreErr){}",
      "var a = 0, b; b = a = a + 1; foo(b);",
      "var a = 0, b; b = a += a + 1; foo(b);",
      "var a = 0, b; b = a++; foo(b);",
      "function foo(a) { var b = a = a + 1; bar(b) } foo();",
      "function foo(a) { var b = a += a + 1; bar(b) } foo();",
      "function foo(a) { var b = a++; bar(b) } foo();",
      "function foo(cb) { cb = function() { function something(a) { cb(1 + a); } register(something); }(); } foo();",
      "function* foo(cb) { cb = yield function(a) { cb(1 + a); }; } foo();",
      "function foo(cb) { cb = tag`hello${function(a) { cb(1 + a); }}`; } foo();",
      "function foo(cb) { var b; cb = b = function(a) { cb(1 + a); }; b(); } foo();",
      "(class { set foo(UNUSED) {} })",
      "class Foo { set bar(UNUSED) {} } console.log(Foo)",
      "var a = function () { a(); }; a();",
      "var a = function(){ return function () { a(); } }; a();",
      "const a = () => { a(); }; a();",
      "const a = () => () => { a(); }; a();",
      r#"export * as ns from "source""#,
      "import.meta",
      "
import { ClassDecoratorFactory } from 'decorators';
@ClassDecoratorFactory()
export class Foo {}
      ",
      "
import { ClassDecorator } from 'decorators';
@ClassDecorator
export class Foo {}
      ",
      "
import { AccessorDecoratorFactory } from 'decorators';
export class Foo {
  @AccessorDecoratorFactory(true)
  get bar() {}
}
      ",
      "
import { AccessorDecorator } from 'decorators';
export class Foo {
  @AccessorDecorator
  set bar() {}
}
      ",
      "
import { MethodDecoratorFactory } from 'decorators';
export class Foo {
  @MethodDecoratorFactory(false)
  bar() {}
}
      ",
      "
import { MethodDecorator } from 'decorators';
export class Foo {
  @MethodDecorator
  static bar() {}
}
      ",
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
      "
import { ConstructorParameterDecorator } from 'decorators';
export class Foo {
  constructor(@ConstructorParameterDecorator bar) {
    this.bar = bar;
  }
}
      ",
      "
import { ParameterDecoratorFactory } from 'decorators';
export class Qux {
  bar(@ParameterDecoratorFactory(true) baz: number) {
    console.log(baz);
  }
}
      ",
      "
import { ParameterDecorator } from 'decorators';
export class Foo {
  static greet(@ParameterDecorator name: string) {
    return name;
  }
}
      ",
      "
import { Input, Output, EventEmitter } from 'decorators';
export class SomeComponent {
  @Input() data;
  @Output()
  click = new EventEmitter();
}
      ",
      "
import { configurable } from 'decorators';
export class A {
  @configurable(true) static prop1;
  @configurable(false)
  static prop2;
}
      ",
      "
import { foo, bar } from 'decorators';
export class B {
  @foo x;
  @bar
  y;
}
      ",
      "
interface Base {}
class Thing implements Base {}
new Thing();
      ",
      "
interface Base {}
const a: Base = {};
console.log(a);
      ",
      "
import { Foo } from 'foo';
function bar<T>() {}
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
const bar = function <T>() {};
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
const bar = <T>() => {};
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
<Foo>(<T>() => {})();
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable<string> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable | undefined = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable & undefined = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther[]> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<Array<SomeOther>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Nullable> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable[] = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Nullable[]> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Array<Nullable>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Array<Nullable<SomeOther>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo implements Component<Nullable> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo extends Component<Nullable, {}> {}
new Foo();
          ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo extends Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component, Component2 } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}>, Component2 {}
new Foo();
      ",
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
      "
import { Nullable } from 'nullable';
function foo(a: Nullable) {
  console.log(a);
}
foo();
      ",
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
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
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  other: Nullable<Another>;
}
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  do(a: Nullable<Another>);
}
      ",
      "
import { Foo } from './types';
class Bar<T extends Foo> {}
new Bar<number>();
      ",
      "
import { Foo, Bar } from './types';
class Baz<T extends Foo & Bar> {}
new Baz<any>();
      ",
      "
import { Foo } from './types';
class Bar<T = Foo> {}
new Bar<number>();
      ",
      "
import { Foo } from './types';
class Foo<T = any> {}
new Foo();
      ",
      "
import { Foo } from './types';
class Foo<T = {}> {}
new Foo();
      ",
      "
import { Foo } from './types';
class Foo<T extends {} = {}> {}
new Foo();
      ",
      "
type Foo = 'a' | 'b' | 'c';
type Bar = number;
export const map: { [name in Foo]: Bar } = {
  a: 1,
  b: 2,
  c: 3,
};
      ",
      "
import { Nullable } from 'nullable';
class A<T> {
  bar: T;
}
new A<Nullable>();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
function foo<T extends Nullable>() {}
foo<SomeOther>();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
class A<T extends Nullable> {
  bar: T;
}
new A<SomeOther>()
      ",
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
      "
import firebase, { User } from 'firebase/app';
// initialize firebase project
firebase.initializeApp({});
export function authenticated(cb: (user: User | null) => void): void {
  firebase.auth().onAuthStateChanged(user => cb(user));
}
      ",
      "
import { Foo } from './types';
export class Bar<T extends Foo> {}
      ",
      "
import webpack from 'webpack';
export default function webpackLoader(this: webpack.loader.LoaderContext) {}
      ",
      "
import execa, { Options as ExecaOptions } from 'execa';
export function foo(options: ExecaOptions): execa {
  options();
}
      ",
      "
import { Foo, Bar } from './types';
export class Baz<F = Foo & Bar> {}
      ",
      "
// warning 'B' is defined but never used
export const a: Array<{ b: B }> = [];
      ",
      "
export enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
      ",
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds;
}
      ",
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds.EMAIL;
}
      ",
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
      "
declare function foo();
      ",
      "
declare namespace Foo {
  function bar(line: string, index: number | null, tabSize: number): number;
  var baz: string;
}
      ",
      "
declare var Foo: {
  new (value?: any): Object;
  foo(): string;
};
      ",
      "
declare class Foo {
  constructor(value?: any): Object;
  foo(): string;
}
      ",
      "
import foo from 'foo';
export interface Bar extends foo.i18n {}
      ",
      "
import foo from 'foo';
import bar from 'foo';
export interface Bar extends foo.i18n<bar> {}
      ",

      // TODO(kdy1): Unignore
      //       "
      // import { TypeA } from './interface';
      // export const a = <GenericComponent<TypeA> />;
      //       ",

      // TODO(kdy1): Unignore
      //       "
      // const text = 'text';
      // export function Foo() {
      //   return (
      //     <div>
      //       <input type=\"search\" size={30} placeholder={text} />
      //     </div>
      //   );
      // }
      //       ",
      "
import { observable } from 'mobx';
export default class ListModalStore {
  @observable
  orderList: IObservableArray<BizPurchaseOrderTO> = observable([]);
}
      ",
      "
import { Dec, TypeA, Class } from 'test';
export default class Foo {
  constructor(
    @Dec(Class)
    private readonly prop: TypeA<Class>,
  ) {}
}
      ",
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
      "export function foo(msg: string): void",
      "function _foo(msg?: string): void",
      "const key = 0; export const obj = { [key]: true };",
      "export class Foo { bar(msg: string): void; }",
    };
  }

  #[test]
  fn no_unused_vars_invalid() {
    assert_lint_err! {
      NoUnusedVars,
      "var a = 0": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      // variable shadowing
      "var a = 1; function foo() { var a = 2; console.log(a); }; use(foo);": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function foox() { return foox(); }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
        }
      ],
      "(function() { function foox() { if (true) { return foox(); } } }())": [
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
        }
      ],
      "var a=10": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a *= 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        }
      ],
      "function f() { var a = 1; return function(){ f(++a); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        }
      ],
      "function foo(first, second) {\ndoStuff(function()\
       {\nconsole.log(second);});};": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
        },
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "first"),
        }
      ],
      "var a=10;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a=10; a=20;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a=10; (function() { var a = 1; alert(a); })();": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a=10, b=0, c=null; alert(a+b)": [
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() { var b=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() \
      { var b=2; var c=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        },
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "function f(){var a=[];return a.map(function(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        }
      ],
      "function f(){var a=[];return a.map(function g(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        }
      ],
      "function f(){var x;function a(){x=42;}function b(){alert(x);}}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        },
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        },
        {
          col: 47,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "function f(a) {}; f();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function a(x, y, z){ return y; }; a();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
        },
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "z"),
        }
      ],
      "var min = Math.min": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
        }
      ],
      "var min = {min: 1}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
        }
      ],
      "Foo.bar = function(baz) { return 1; };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "baz"),
        }
      ],
      "var min = {min: 1}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
        }
      ],
      "function gg(baz, bar) { return baz; }; gg();": [
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
        }
      ],
      "(function(foo, baz, bar) { return baz; })();": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
        },
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
        }
      ],
      "(function z(foo) { var bar = 33; })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
        }
      ],
      "(function z(foo) { z(); })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a = 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
        },
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "import x from \"y\";": [
        {
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
        }
      ],
      "export function fn2({ x, y }) {\n console.log(x); \n};": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
        }
      ],
      "export function fn2( x, y ) {\n console.log(x); \n};": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
        }
      ],
      "var _a; var b;": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "function foo(a, _b) { } foo()": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function foo(a, _b, c) { return a; } foo();": [
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "const data = { type: 'coords', x: 1, y: 2 };\
     const { type, ...coords } = data;\n console.log(coords);": [
        {
          col: 52,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "type"),
        }
      ],
      "const data = { type: 'coords', x: 3, y: 2 };\
        const { type, ...coords } = data;\n console.log(type)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "coords"),
        }
      ],
      "const data = { vars: \
      ['x','y'], x: 1, y: 2 }; const { vars: [x], ...coords } = data;\n\
       console.log(coords)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
        }
      ],
      "const data = { defaults: { x: 0 }, x: 1, y: 2 }; const { defaults: { x }, ...coords } = data;\n console.log(coords)": [
        {
          col: 69,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
        }
      ],
      "export default function(a) {}": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "export default function(a, b) { console.log(a); }": [
        {
          col: 27,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "export default (function(a) {});": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "export default (function(a, b) { console.log(a); });": [
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "export default (a) => {};": [
        {
          col: 16,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "export default (a, b) => { console.log(a); };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
        }
      ],
      "try{}catch(err){};": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "err"),
        }
      ],
      "(function ({ a }, b ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "(function ({ a }, { b, c } ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "(function ([ a ], b ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "(function ([ a ], [ b, c ] ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "var a = function() { a(); };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = function(){ return function() { a(); } };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "const a = () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "const a = () => () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = function() { a(); };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = function(){ return function() { a(); } };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "const a = () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "const a = () => () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "let a = 'a';
    a = 10;
    function foo(){
        a = 11;
        a = () => {
            a = 13
        }
    }": [
        {
          line: 1,
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        },
        {
          line: 3,
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
        }
      ],
      "let c = 'c'
    c = 10
    function foo1() {
      c = 11
      c = () => {
        c = 13
      }
    }
    c = foo1": [
        {
          line: 1,
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
        }
      ],
      "
import { ClassDecoratorFactory } from 'decorators';
export class Foo {}
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "ClassDecoratorFactory"),
        }
      ],
      "
import { Foo, Bar } from 'foo';
function baz<Foo>() {}
baz<Bar>();
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Foo"),
        }
      ],
      "
import { Nullable } from 'nullable';
const a: string = 'hello';
console.log(a);
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<string> = 'hello';
console.log(a);
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do = (a: Nullable) => {
    console.log(a);
  };
}
new A();
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(a: Nullable) {
    console.log(a);
  }
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(): Nullable {
    return null;
  }
}
new A();
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
interface A {
  do(a: Nullable);
}
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
interface A {
  other: Nullable;
}
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
function foo(a: string) {
  console.log(a);
}
foo();
        ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
function foo(): string | null {
  return null;
}
foo();
        ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
class A extends Nullable {
  other: Nullable<Another>;
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
abstract class A extends Nullable {
  other: Nullable<Another>;
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
        }
      ],
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
        ": [
        {
          line: 2,
          col: 5,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "FormFieldIds"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export interface Bar extends baz.test {}
        ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
        }
      ]
    };
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_for_loop_control_flow() {
    assert_lint_err! {
      NoUnusedVars,
      "(function(obj) { var name; for ( name in obj ) { i(); return; } })({});": [
        {
          col: 21,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
        }
      ],
      "(function(obj) { var name; for ( name in obj ) { } })({});": [
        {
          col: 21,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
        }
      ],
      "(function(obj) { for ( var name in obj ) { } })({});": [
        {
          col: 37,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
        }
      ]
    };
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_expr() {
    assert_lint_err! {
      NoUnusedVars,
      "var a = 0; a = a + 1;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = 0; a = a + a;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = 0; a += a + 1": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = 0; a++;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function foo(a) { a = a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function foo(a) { a += a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "function foo(a) { a++ } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = 3; a = a * 5 + 6;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "var a = 2, b = 4; a = a * 2 + b;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ],
      "const a = 1; a += 1;": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
        }
      ]
    };
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_to_self() {
    assert_lint_err! {
      NoUnusedVars,
      "function foo(cb) { cb = function(a) { cb(1 + a); }; bar(not_cb); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
        }
      ],
      "function foo(cb) { cb = function(a) { return cb(1 + a); }(); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
        }
      ],
      "function foo(cb) { cb = (function(a) { cb(1 + a); }, cb); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
        }
      ],
      "function foo(cb) { cb = (0, function(a) { cb(1 + a); }); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
        }
      ]
    };
  }

  #[test]
  #[ignore = "pure method analysis is not implemented yet"]
  fn no_unused_vars_err_array_methods() {
    assert_lint_err! {
      NoUnusedVars,
      "let myArray = [1,2,3,4].filter((x) => x == 0); myArray = myArray.filter((x) => x == 1);": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "myArray"),
        }
      ]
    };
  }

  #[test]
  #[ignore = "swc cannot parse this at the moment"]
  fn no_unused_vars_ts_err_06() {
    assert_lint_err! {
      NoUnusedVars,
      "
import test from 'test';
import baz from 'baz';
export interface Bar extends baz().test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export class Bar implements baz.test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export class Bar implements baz().test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
        }
      ]
    };
  }

  #[test]
  #[ignore = "typescript property analysis is not implemented yet"]
  fn no_unused_vars_ts_ok_12() {
    assert_lint_ok! {
      NoUnusedVars,
      "
export class App {
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(bar: string);
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(baz: string, private logger: Logger) {
    console.log(baz);
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(baz: string, private logger: Logger, private bar: () => void) {
    console.log(this.logger);
    this.bar();
  }
}
      ",
      "
export class App {
  constructor(private logger: Logger) {}
  meth() {
    console.log(this.logger);
  }
}
      ",
    };
  }
}
