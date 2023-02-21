// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::program_ref;
use super::{Context, LintRule};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::Id;
use deno_ast::swc::ast::{
  ArrowExpr, AssignPatProp, CallExpr, CatchClause, ClassDecl, ClassMethod,
  ClassProp, Constructor, Decl, DefaultDecl, ExportDecl, ExportDefaultDecl,
  ExportNamedSpecifier, Expr, FnDecl, FnExpr, Function, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  MemberExpr, MemberProp, MethodKind, ModuleExportName, NamedExport, Param,
  Pat, PrivateMethod, Prop, PropName, SetterProp, TsEntityName, TsEnumDecl,
  TsExprWithTypeArgs, TsInterfaceDecl, TsModuleDecl, TsNamespaceDecl,
  TsPropertySignature, TsTypeAliasDecl, TsTypeQueryExpr, TsTypeRef, VarDecl,
  VarDeclarator,
};
use deno_ast::swc::atoms::js_word;
use deno_ast::swc::utils::find_pat_ids;
use deno_ast::swc::visit::{Visit, VisitWith};
use deno_ast::{MediaType, SourceRangedForSpanned};
use derive_more::Display;
use if_chain::if_chain;
use std::collections::HashSet;
use std::iter;

#[derive(Debug)]
pub struct NoUnusedVars;

const CODE: &str = "no-unused-vars";

#[derive(Display)]
enum NoUnusedVarsMessage {
  #[display(fmt = "`{}` is never used", _0)]
  NeverUsed(String),
}

#[derive(Display)]
enum NoUnusedVarsHint {
  #[display(
    fmt = "If this is intentional, prefix it with an underscore like `_{}`",
    _0
  )]
  AddPrefix(String),
  #[display(
    fmt = "If this is intentional, alias it with an underscore like `{} as _{}`",
    _0,
    _0
  )]
  Alias(String),
}

impl LintRule for NoUnusedVars {
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
    // Skip linting this file to avoid emitting false positives about `jsxFactory` and `jsxFragmentFactory`
    // if it's a JSX or TSX file.
    // See https://github.com/denoland/deno_lint/pull/664#discussion_r614692736
    if is_jsx_file(context.media_type()) {
      return;
    }

    let mut collector = Collector::default();
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut collector),
      ProgramRef::Script(s) => s.visit_with(&mut collector),
    }

    let mut visitor = NoUnusedVarVisitor::new(
      context,
      collector.used_vars,
      collector.used_types,
    );
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unused_vars.md")
  }
}

fn is_jsx_file(media_type: MediaType) -> bool {
  media_type == MediaType::Jsx || media_type == MediaType::Tsx
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
  /// The variable usage during its declaration should _NOT_ be treated as used.
  /// For example:
  ///
  /// ```typescript
  /// // `a` is called, but effectively nothing occurs until `a` is called from _outside_ of this
  /// // function body.
  /// const a = () => { a(); };
  ///
  /// // Same goes for type or interface definitions.
  /// type JsonValue = number | string | boolean | Array<JsonValue> | {
  ///   [key: string]: JsonValue;
  /// };
  /// interface Foo {
  ///   a: Foo;
  /// }
  /// ```
  ///
  /// To handle it, we need to store the variables that are currently being declared.
  /// This is a helper method, responsible for preserving and then restoring variables data.
  fn with_cur_defining<I, F>(&mut self, ids: I, op: F)
  where
    I: IntoIterator<Item = Id>,
    F: FnOnce(&mut Collector),
  {
    // Preserve the original state
    let prev_len = self.cur_defining.len();
    self.cur_defining.extend(ids);

    op(self);

    // Restore the original state
    self.cur_defining.drain(prev_len..);
    assert_eq!(self.cur_defining.len(), prev_len);
  }

  /// This is a helper method, responsible for temporarily ignoring `cur_defining` while doing
  /// the given operation (`op`).
  ///
  /// For some context, we need to ignore variables that are being declared (which we call
  /// `cur_defining`).
  /// Take function arguments as an example. If `cur_defining` is used inside the arguments, we
  /// have to think of it as _used_.
  ///
  /// ```typescript
  /// const i = setInterval(() => {
  ///  clearInterval(i);
  /// }, 1000);
  /// ```
  ///
  /// In the above example, when visiting `setInterval`, we have `i` included in `cur_defining`.
  /// `setInterval` is taking a closure as an argument and `i` is used in it.
  /// Naturally we have to treat `i` as used, because this closure is effectively invoked
  /// lazily; not invoked at the time when `i` is being defined.
  fn without_cur_defining<F>(&mut self, op: F)
  where
    F: FnOnce(&mut Collector),
  {
    let prev = std::mem::take(&mut self.cur_defining);
    op(self);
    self.cur_defining = prev;
  }

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
  fn visit_class_prop(&mut self, n: &ClassProp) {
    n.decorators.visit_with(self);

    if let PropName::Computed(_) = &n.key {
      n.key.visit_with(self);
    }

    n.value.visit_with(self);
    n.type_ann.visit_with(self);
  }

  fn visit_ts_property_signature(&mut self, n: &TsPropertySignature) {
    if n.computed {
      n.key.visit_with(self);
    }

    n.type_params.visit_with(self);
    n.type_ann.visit_with(self);
    n.params.visit_with(self);
    n.init.visit_with(self);
  }

  fn visit_ts_type_ref(&mut self, ty: &TsTypeRef) {
    ty.type_params.visit_with(self);

    let id = get_id(&ty.type_name);
    self.used_types.insert(id);
  }

  fn visit_ts_expr_with_type_args(&mut self, n: &TsExprWithTypeArgs) {
    n.expr.visit_with(self);
    n.type_args.visit_children_with(self);
  }

  fn visit_ts_type_query_expr(&mut self, n: &TsTypeQueryExpr) {
    if let TsTypeQueryExpr::TsEntityName(e) = n {
      let id = get_id(e);
      self.used_vars.insert(id);
    }
    n.visit_children_with(self);
  }

  fn visit_prop(&mut self, n: &Prop) {
    match n {
      Prop::Shorthand(i) => self.mark_as_usage(i),
      _ => n.visit_children_with(self),
    }
  }

  fn visit_prop_name(&mut self, n: &PropName) {
    if let PropName::Computed(computed) = n {
      computed.visit_children_with(self);
    }
    // Don't check Ident, Str, Num and BigInt
  }

  fn visit_expr(&mut self, expr: &Expr) {
    match expr {
      Expr::Ident(i) => self.mark_as_usage(i),
      _ => expr.visit_children_with(self),
    }
  }

  fn visit_pat(&mut self, pat: &Pat) {
    match pat {
      // Ignore patterns
      Pat::Ident(i) => {
        i.type_ann.visit_with(self);
      }
      Pat::Invalid(..) => {}
      //
      _ => pat.visit_children_with(self),
    }
  }

  fn visit_assign_pat_prop(&mut self, assign_pat_prop: &AssignPatProp) {
    // handle codes like `const { foo, bar = foo } = { foo: 42 };`
    self.without_cur_defining(|a| {
      assign_pat_prop.value.visit_children_with(a);
    });
  }

  fn visit_member_expr(&mut self, member_expr: &MemberExpr) {
    member_expr.obj.visit_with(self);
    if let MemberProp::Computed(prop) = &member_expr.prop {
      prop.visit_with(self);
    }
  }

  /// export is kind of usage
  fn visit_export_named_specifier(&mut self, export: &ExportNamedSpecifier) {
    if let ModuleExportName::Ident(ident) = &export.orig {
      self.used_vars.insert(ident.to_id());
    }
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl) {
    let id = decl.ident.to_id();
    self.with_cur_defining(iter::once(id), |a| {
      decl.function.visit_with(a);
    });
  }

  fn visit_fn_expr(&mut self, expr: &FnExpr) {
    // We have to do nothing special for identifiers of FnExprs (if any), because they are allowed
    // to be not-used.
    expr.function.visit_with(self);
  }

  fn visit_function(&mut self, function: &Function) {
    if_chain! {
      if let Some(first_param) = function.params.get(0);
      if let Pat::Ident(ident) = &first_param.pat;
      if ident.type_ann.is_some();
      if ident.id.sym == js_word!("this");
      then {
        // If the first parameter of a function is `this` keyword with type annotated, it is a
        // fake parameter specifying what type `this` becomes inside the function body.
        // (See https://www.typescriptlang.org/docs/handbook/functions.html#this-parameters
        // for more info)
        // Since it's just a fake parameter, we can mark it as used.
        self.mark_as_usage(&ident.id);
      }
    }

    function.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr) {
    call_expr.callee.visit_children_with(self);

    for arg in &call_expr.args {
      self.without_cur_defining(|a| {
        arg.visit_children_with(a);
      });
    }

    call_expr.type_args.visit_children_with(self);
  }

  fn visit_class_decl(&mut self, decl: &ClassDecl) {
    let id = decl.ident.to_id();
    self.with_cur_defining(iter::once(id), |a| {
      decl.class.visit_with(a);
    });
  }

  fn visit_ts_interface_decl(&mut self, decl: &TsInterfaceDecl) {
    let id = decl.id.to_id();
    self.with_cur_defining(iter::once(id), |a| {
      decl.extends.visit_with(a);
      decl.body.visit_with(a);
      if let Some(type_params) = &decl.type_params {
        type_params.visit_with(a);
      }
    });
  }

  fn visit_ts_type_alias_decl(&mut self, decl: &TsTypeAliasDecl) {
    let id = decl.id.to_id();
    self.with_cur_defining(iter::once(id), |a| {
      decl.type_ann.visit_with(a);
      if let Some(type_params) = &decl.type_params {
        type_params.visit_with(a);
      }
    });
  }

  fn visit_ts_enum_decl(&mut self, decl: &TsEnumDecl) {
    let id = decl.id.to_id();
    self.with_cur_defining(iter::once(id), |a| {
      decl.members.visit_with(a);
    });
  }

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
    let declaring_ids: Vec<Id> = find_pat_ids(&declarator.name);
    self.with_cur_defining(declaring_ids, |a| {
      declarator.name.visit_with(a);
      declarator.init.visit_with(a);
    });
  }
}

fn get_id(r: &TsEntityName) -> Id {
  match r {
    TsEntityName::TsQualifiedName(q) => get_id(&q.left),
    TsEntityName::Ident(i) => i.to_id(),
  }
}

struct NoUnusedVarVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  used_vars: HashSet<Id>,
  used_types: HashSet<Id>,
}

impl<'c, 'view> NoUnusedVarVisitor<'c, 'view> {
  fn new(
    context: &'c mut Context<'view>,
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

#[derive(Debug, Clone, Copy)]
enum IdentKind<'a> {
  NamedImport(&'a Ident),
  DefaultImport(&'a Ident),
  StarAsImport(&'a Ident),
  Other(&'a Ident),
}

impl<'a> IdentKind<'a> {
  fn inner(&self) -> &Ident {
    match *self {
      IdentKind::NamedImport(ident) => ident,
      IdentKind::DefaultImport(ident) => ident,
      IdentKind::StarAsImport(ident) => ident,
      IdentKind::Other(ident) => ident,
    }
  }

  fn to_message(self) -> NoUnusedVarsMessage {
    let ident = self.inner();
    NoUnusedVarsMessage::NeverUsed(ident.sym.to_string())
  }

  fn to_hint(self) -> NoUnusedVarsHint {
    let symbol = self.inner().sym.to_string();
    match self {
      IdentKind::NamedImport(_) => NoUnusedVarsHint::Alias(symbol),
      IdentKind::DefaultImport(_)
      | IdentKind::StarAsImport(_)
      | IdentKind::Other(_) => NoUnusedVarsHint::AddPrefix(symbol),
    }
  }
}

impl<'c, 'view> NoUnusedVarVisitor<'c, 'view> {
  fn handle_id(&mut self, ident: IdentKind) {
    let inner = ident.inner();
    if inner.sym.starts_with('_') {
      return;
    }

    if !self.used_vars.contains(&inner.to_id()) {
      // The variable is not used.
      self.context.add_diagnostic_with_hint(
        inner.range(),
        CODE,
        ident.to_message(),
        ident.to_hint(),
      );
    }
  }
}

impl<'c, 'view> Visit for NoUnusedVarVisitor<'c, 'view> {
  fn visit_arrow_expr(&mut self, expr: &ArrowExpr) {
    let declared_idents: Vec<Ident> = find_pat_ids(&expr.params);

    for ident in declared_idents {
      self.handle_id(IdentKind::Other(&ident));
    }
    expr.body.visit_with(self)
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl) {
    if decl.declare {
      return;
    }

    self.handle_id(IdentKind::Other(&decl.ident));

    // If function body is not present, it's an overload definition
    if decl.function.body.is_some() {
      decl.function.visit_with(self);
    }
  }

  fn visit_var_decl(&mut self, n: &VarDecl) {
    if n.declare {
      return;
    }

    n.decls.visit_with(self);
  }

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
    let declared_idents: Vec<Ident> = find_pat_ids(&declarator.name);

    for ident in declared_idents {
      self.handle_id(IdentKind::Other(&ident));
    }
    declarator.name.visit_with(self);
    declarator.init.visit_with(self);
  }

  fn visit_class_decl(&mut self, n: &ClassDecl) {
    if n.declare {
      return;
    }

    self.handle_id(IdentKind::Other(&n.ident));
    n.visit_children_with(self);
  }

  fn visit_catch_clause(&mut self, clause: &CatchClause) {
    let declared_idents: Vec<Ident> = find_pat_ids(&clause.param);

    for ident in declared_idents {
      self.handle_id(IdentKind::Other(&ident));
    }

    clause.body.visit_with(self);
  }

  fn visit_setter_prop(&mut self, prop: &SetterProp) {
    prop.key.visit_with(self);
    prop.body.visit_with(self);
  }

  fn visit_constructor(&mut self, constructor: &Constructor) {
    // If function body is not present, it's an overload definition
    if constructor.body.is_none() {
      return;
    }

    constructor.visit_children_with(self);
  }

  fn visit_class_method(&mut self, method: &ClassMethod) {
    method.function.decorators.visit_with(self);
    method.key.visit_with(self);

    // If method body is not present, it's an overload definition
    if matches!(method.kind, MethodKind::Method if method.function.body.is_some())
    {
      method.function.params.visit_children_with(self);
    }

    method.function.body.visit_with(self);
  }

  fn visit_private_method(&mut self, method: &PrivateMethod) {
    method.function.decorators.visit_with(self);
    method.key.visit_with(self);

    // If method body is not present, it's an overload definition
    if method.function.body.is_some() {
      method.function.params.visit_children_with(self);
    }

    method.function.body.visit_with(self);
  }

  fn visit_param(&mut self, param: &Param) {
    let declared_idents: Vec<Ident> = find_pat_ids(&param.pat);

    for ident in declared_idents {
      self.handle_id(IdentKind::Other(&ident));
    }
    param.visit_children_with(self);
  }

  fn visit_import_named_specifier(&mut self, import: &ImportNamedSpecifier) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }
    self.handle_id(IdentKind::NamedImport(&import.local));
  }

  fn visit_import_default_specifier(
    &mut self,
    import: &ImportDefaultSpecifier,
  ) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }

    self.handle_id(IdentKind::DefaultImport(&import.local));
  }

  fn visit_import_star_as_specifier(&mut self, import: &ImportStarAsSpecifier) {
    if self.used_types.contains(&import.local.to_id()) {
      return;
    }
    self.handle_id(IdentKind::StarAsImport(&import.local));
  }

  /// No error as export is kind of usage
  fn visit_export_decl(&mut self, export: &ExportDecl) {
    match &export.decl {
      Decl::Class(c) if !c.declare => {
        c.class.visit_with(self);
      }
      Decl::Fn(f) if !f.declare => {
        // If function body is not present, it's an overload definition
        if f.function.body.is_some() {
          f.function.visit_with(self);
        }
      }
      Decl::Var(v) if !v.declare => {
        for decl in &v.decls {
          decl.name.visit_with(self);
          decl.init.visit_with(self);
        }
      }
      _ => {}
    }
  }

  fn visit_export_default_decl(&mut self, export: &ExportDefaultDecl) {
    match &export.decl {
      DefaultDecl::Class(c) => {
        c.class.visit_with(self);
      }
      DefaultDecl::Fn(f) => {
        // If function body is not present, it's an overload definition
        if f.function.body.is_some() {
          f.function.visit_with(self);
        }
      }
      DefaultDecl::TsInterfaceDecl(i) => {
        i.visit_children_with(self);
      }
    }
  }

  fn visit_params(&mut self, params: &[Param]) {
    match params.first() {
      Some(Param {
        pat: Pat::Ident(i), ..
      }) if i.id.sym == *"this" => params
        .iter()
        .skip(1)
        .for_each(|param| param.visit_with(self)),
      _ => params.iter().for_each(|param| param.visit_with(self)),
    }
  }

  fn visit_ts_enum_decl(&mut self, n: &TsEnumDecl) {
    if n.declare {
      return;
    }

    if self.used_types.contains(&n.id.to_id()) {
      return;
    }
    self.handle_id(IdentKind::Other(&n.id));
  }

  fn visit_ts_module_decl(&mut self, n: &TsModuleDecl) {
    if n.declare {
      return;
    }

    n.body.visit_with(self);
  }

  fn visit_ts_namespace_decl(&mut self, n: &TsNamespaceDecl) {
    if n.declare {
      return;
    }

    n.body.visit_with(self);
  }

  /// no-op as export is kind of usage
  fn visit_named_export(&mut self, _: &NamedExport) {}
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
      "var a = 1; console.log?.(a)",
      "var a = 1; a?.()",
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

      // https://github.com/denoland/deno_lint/issues/670
      "export declare class Foo { constructor(arg: string); }",
      "export declare function foo(): void;",
      "export declare const foo: number;",
      "export declare let foo: number;",
      "export declare var foo: number;",

      "
import foo from 'foo';
export interface Bar extends foo.i18n {}
      ",
      "
import foo from 'foo';
import bar from 'foo';
export interface Bar extends foo.i18n<bar> {}
      ",
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
      "export default function foo(msg: string): void",
      "function _foo(msg?: string): void",
      "const key = 0; export const obj = { [key]: true };",
      "export class Foo { bar(msg: string): void; }",
      "import { foo } from './foo.ts'; type Bar = typeof foo;",
      "interface Foo {} export interface Bar extends Foo {}",
      "import type Foo from './foo.ts'; export interface Bar extends Foo {}",
      "import type Foo from './foo.ts'; export class Bar implements Foo {}",
      "import type Foo from './foo.ts'; interface _Bar<T extends Foo> {}",
      "import type Foo from './foo.ts'; type _Bar<T extends Foo> = T;",
      "type Foo = { a: number }; function _bar<T extends keyof Foo>() {}",

      // https://github.com/denoland/deno_lint/issues/667#issuecomment-821856328
      // `this` as a fake parameter. See: https://www.typescriptlang.org/docs/handbook/functions.html#this-parameters
      "export function f(this: void) {}",
      "export const foo = { bar(this: Foo) {} };",
      "export interface Foo { bar(this: void): void; }",
      "export interface Foo { bar(baz: (this: void) => void ): void; }",
      "export class Foo { bar(this: Foo) {} }",
      r#"
export abstract class Point4DPartial {
    toString(this: Point4D): string {
      return [this.getPosition(), this.z, this.getTime()].join(", ");
    }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/667
      "const i = setInterval(() => clearInterval(i), 1000);",
      "const i = setInterval(function() { clearInterval(i); }, 1000);",
      "const i = setInterval(function foo() { clearInterval(i); }, 1000);",
      "setTimeout(function foo() { const foo = 42; console.log(foo); });",
      "const fn = function foo() {}; fn();",

      // https://github.com/denoland/deno_lint/issues/687
      "const { foo, bar = foo } = { foo: 42 }; console.log(bar);",
      "const { foo, bar = f(foo) } = makeObj(); console.log(bar);",
      "const { foo, bar = foo.prop } = makeObj(); console.log(bar);",
      "const { foo, bar = await foo } = makeObj(); console.log(bar);",
      "const { foo, bar = foo ? 42 : 7 } = makeObj(); console.log(bar);",
      "const { foo, bar = baz ? foo : 7 } = makeObj(); console.log(bar);",
      "const { foo, bar = `hello ${foo}` } = makeObj(); console.log(bar);",
      "const { foo, bar = [foo] } = makeObj(); console.log(bar);",
      "const { foo, bar = { key: foo } } = makeObj(); console.log(bar);",
      "const { foo, bar = function() { foo(); } } = makeObj(); console.log(bar);",
      "const { foo, bar = () => foo() } = makeObj(); console.log(bar);",

      // https://github.com/denoland/deno_lint/issues/690
      r#"
export class Foo {
  constructor(x: number);
  constructor(y: string);
  constructor(xy: number | string) {
    console.log(xy);
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/705
      r#"
type Foo = string[];
function _bar(...Foo: Foo): void {
  console.log(Foo);
}
      "#,
      r#"
import type { Foo } from "./foo.ts";
function _bar(...Foo: Foo) {
  console.log(Foo);
}
      "#,
      r#"
import type { Filters } from "./types.ts";
export function audioFilters(...Filters: Filters[]): void {
  for (const filter of Filters) {
    console.log(filter);
    return;
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/739
      r#"
export class Test {
  #myFunction(value: string): string;
  #myFunction(value: number): number;
  #myFunction(value: string | number) {
    return value;
  }
}
      "#,
    };

    // JSX or TSX
    assert_lint_ok! {
      NoUnusedVars,
      filename: "foo.tsx",
      r#"
import { TypeA } from './interface';
export const a = <GenericComponent<TypeA> />;
      "#,
      r#"
const text = 'text';
export function Foo() {
  return (
    <div id="hoge">
      <input type="search" size={30} placeholder={text} />
    </div>
  );
}
      "#,
      r#"
function Root() { return null; }
function Child() { return null; }
export default <Root><Child>Hello World!</Child></Root>;
      "#,

      // https://github.com/denoland/deno_lint/issues/663
      r#"
import React from "./dummy.ts";
export default <div />;
      "#,
      r#"
import React from "./dummy.ts";
function Component() { return null; }
export default <Component />;
      "#,
      r#"
import React from "./dummy.ts";
const Component = () => { return null; }
export default <Component />;
      "#,
      r#"
import React from "./dummy.ts";
class Component extends React.Component { render() { return null; } }
export default <Component />;
      "#,
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      // variable shadowing
      "var a = 1; function foo() { var a = 2; console.log(a); }; use(foo);": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foox() { return foox(); }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foox"),
        }
      ],
      "class Foo {}": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "Foo"),
        }
      ],
      "(function() { function foox() { if (true) { return foox(); } } }())": [
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foox"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a *= 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f() { var a = 1; return function(){ f(++a); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function foo(first, second) {\ndoStuff(function()\
       {\nconsole.log(second);});};": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "first"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "first"),
        }
      ],
      "var a=10; a=20;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a=10; (function() { var a = 1; alert(a); })();": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a=10, b=0, c=null; alert(a+b)": [
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() { var b=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() \
      { var b=2; var c=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        },
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "function f(){var a=[];return a.map(function(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f(){var a=[];return a.map(function g(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f(){var x;function a(){x=42;}function b(){alert(x);}}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        },
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          col: 47,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "function f(a) {}; f();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function a(x, y, z){ return y; }; a();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        },
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "z"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "z"),
        }
      ],
      "var min = Math.min": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "min"),
        }
      ],
      "Foo.bar = function(baz) { return 1; };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "baz"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "baz"),
        }
      ],
      "var min = {min: 1}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "min"),
        }
      ],
      "function gg(baz, bar) { return baz; }; gg();": [
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function(foo, baz, bar) { return baz; })();": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function z(foo) { var bar = 33; })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function z(foo) { z(); })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a = 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        },
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "import x from \"y\";": [
        {
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "export function fn2({ x, y }) {\n console.log(x); \n};": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "y"),
        }
      ],
      "export function fn2( x, y ) {\n console.log(x); \n};": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "y"),
        }
      ],
      "var _a; var b;": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "function foo(a, _b) { } foo()": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a, _b, c) { return a; } foo();": [
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "const data = { type: 'coords', x: 1, y: 2 };\
     const { type, ...coords } = data;\n console.log(coords);": [
        {
          col: 52,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "type"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "type"),
        }
      ],
      "const data = { type: 'coords', x: 3, y: 2 };\
        const { type, ...coords } = data;\n console.log(type)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "coords"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "coords"),
        }
      ],
      "const data = { vars: \
      ['x','y'], x: 1, y: 2 }; const { vars: [x], ...coords } = data;\n\
       console.log(coords)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "const data = { defaults: { x: 0 }, x: 1, y: 2 }; const { defaults: { x }, ...coords } = data;\n console.log(coords)": [
        {
          col: 69,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "export default function(a) {}": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default function(a, b) { console.log(a); }": [
        {
          col: 27,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "export default (function(a) {});": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default (function(a, b) { console.log(a); });": [
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "export default (a) => {};": [
        {
          col: 16,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default (a, b) => { console.log(a); };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "try{}catch(err){};": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "err"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "err"),
        }
      ],
      "(function ({ a }, b ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "(function ({ a }, { b, c } ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "var a = function() { a(); };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = function(){ return function() { a(); } };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = () => () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          line: 3,
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "ClassDecoratorFactory"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Foo"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
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
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "FormFieldIds"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
      "
import React from './dummy.ts';
const a = 42;
foo(a);
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "React"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "React"),
        }
      ],

      // https://github.com/denoland/deno_lint/issues/730
      r#"import * as foo from "./foo.ts";"#: [
        {
          line: 1,
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],

      // FnExpr
      "const fn = function foo() { foo(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "fn"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "fn"),
        }
      ],

      // https://github.com/denoland/deno_lint/issues/697
      "for (const x of xs) {}": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for (x of xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for await (const x of xs) {}": [
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for await (x of xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for (const x in xs) {}": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for (x in xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for (const [x1, x2] of xs) {}": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
        {
          col: 16,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        }
      ],
      "let x1, x2; for ([x1, x2] of xs) { console.log(x1); }": [
        {
          col: 8,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        },
      ],
      "for (const { x1, x2 } of xs) {}": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        }
      ],
      "let x1, x2; for ({ x1, x2 } of xs) { console.log(x2); }": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
      ],

        r#"
export class Foo {
  #myFunction(value: string): string;
  #myFunction(value: number): number;
  #myFunction(value: string | number) {
    return 42;
  }
}
        "#: [
          {
            col: 14,
            line:5,
            message: variant!(NoUnusedVarsMessage, NeverUsed, "value"),
            hint: variant!(NoUnusedVarsHint, AddPrefix, "value"),
          },
        ],
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
        }
      ],
      "(function(obj) { var name; for ( name in obj ) { } })({});": [
        {
          col: 21,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
        }
      ],
      "(function(obj) { for ( var name in obj ) { } })({});": [
        {
          col: 37,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a = a + a;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a += a + 1": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a++;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a = a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a += a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a++ } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 3; a = a * 5 + 6;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 2, b = 4; a = a * 2 + b;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = 1; a += 1;": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = function(a) { return cb(1 + a); }(); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = (function(a) { cb(1 + a); }, cb); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = (0, function(a) { cb(1 + a); }); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "myArray"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
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
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
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
