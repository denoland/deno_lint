// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::HashMap;
use std::sync::Arc;
use swc_atoms::JsWord;
use swc_common::Span;
use swc_ecmascript::ast::{
  AssignExpr, BlockStmt, Ident, Module, ObjectPatProp, Pat, PatOrExpr, VarDecl,
  VarDeclKind,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::{Node, Visit, VisitWith};

pub struct PreferConst;

impl LintRule for PreferConst {
  fn new() -> Box<Self> {
    Box::new(PreferConst)
  }

  fn code(&self) -> &'static str {
    "prefer-const"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = PreferConstVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

enum Initalized {
  SameScope,
  DifferentScope,
  NotYet,
}

struct VarStatus {
  initialized: Initalized,
  reassigned: bool,
}

impl VarStatus {
  fn should_report(&self) -> bool {
    use Initalized::*;
    match self.initialized {
      DifferentScope | NotYet => false,
      SameScope => !self.reassigned,
    }
  }
}

struct PreferConstVisitor {
  symbols: HashMap<JsWord, Vec<VarStatus>>,
  vars_declareted_per_scope: Vec<HashMap<JsWord, Span>>,
  context: Arc<Context>,
}

impl PreferConstVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self {
      context,
      symbols: HashMap::new(),
      vars_declareted_per_scope: Vec::new(),
    }
  }

  fn report(&self, sym: JsWord, span: Span) {
    self.context.add_diagnostic(
      span,
      "prefer-const",
      &format!(
        "'{}' is never reassigned. Use 'const' instead",
        sym.to_string()
      ),
    );
  }

  fn insert_var(&mut self, ident: &Ident, has_init: bool) {
    self
      .vars_declareted_per_scope
      .last_mut()
      .unwrap()
      .entry(ident.sym.clone())
      .or_insert(ident.span);

    self
      .symbols
      .entry(ident.sym.clone())
      .or_default()
      .push(VarStatus {
        initialized: if has_init {
          Initalized::SameScope
        } else {
          Initalized::NotYet
        },
        reassigned: false,
      });
  }

  fn mark_reassigned(&mut self, ident: &Ident) {
    dbg!(ident);
    let status = self
      .symbols
      .get_mut(&ident.sym)
      .unwrap()
      .last_mut()
      .unwrap();

    use Initalized::*;
    if self
      .vars_declareted_per_scope
      .last()
      .unwrap()
      .contains_key(&ident.sym)
    {
      match status.initialized {
        NotYet => {
          status.initialized = SameScope;
        }
        _ => {
          status.reassigned = true;
        }
      }
    } else {
      match status.initialized {
        NotYet => {
          status.initialized = DifferentScope;
        }
        _ => {
          status.reassigned = true;
        }
      }
    }
  }

  fn extract_decl_idents(&mut self, pat: &Pat, has_init: bool) {
    match pat {
      Pat::Ident(ident) => self.insert_var(ident, has_init),
      Pat::Array(array_pat) => {
        for elem in &array_pat.elems {
          if let Some(elem_pat) = elem {
            self.extract_decl_idents(elem_pat, has_init);
          }
        }
      }
      Pat::Rest(rest_pat) => self.extract_decl_idents(&*rest_pat.arg, has_init),
      Pat::Object(object_pat) => {
        for prop in &object_pat.props {
          match prop {
            ObjectPatProp::KeyValue(key_value) => {
              self.extract_decl_idents(&*key_value.value, has_init)
            }
            ObjectPatProp::Assign(assign) => {
              if assign.value.is_some() {
                self.insert_var(&assign.key, true);
              } else {
                self.insert_var(&assign.key, has_init);
              }
            }
            ObjectPatProp::Rest(rest) => {
              self.extract_decl_idents(&*rest.arg, has_init)
            }
          }
        }
      }
      Pat::Assign(assign_pat) => {
        self.extract_decl_idents(&*assign_pat.left, true)
      }
      _ => {}
    }
  }

  fn extract_assign_idents(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => self.mark_reassigned(ident),
      Pat::Array(array_pat) => {
        for elem in &array_pat.elems {
          if let Some(elem_pat) = elem {
            self.extract_assign_idents(elem_pat);
          }
        }
      }
      Pat::Rest(rest_pat) => self.extract_assign_idents(&*rest_pat.arg),
      Pat::Object(object_pat) => {
        for prop in &object_pat.props {
          match prop {
            ObjectPatProp::KeyValue(key_value) => {
              self.extract_assign_idents(&*key_value.value)
            }
            ObjectPatProp::Assign(assign) => {
              if assign.value.is_some() {
                self.mark_reassigned(&assign.key);
              } else {
                self.mark_reassigned(&assign.key);
              }
            }
            ObjectPatProp::Rest(rest) => self.extract_assign_idents(&*rest.arg),
          }
        }
      }
      Pat::Assign(assign_pat) => self.extract_assign_idents(&*assign_pat.left),
      _ => {}
    }
  }

  fn enter_scope(&mut self) {
    self.vars_declareted_per_scope.push(HashMap::new());
  }

  fn exit_scope(&mut self) {
    let cur_scope_vars = self.vars_declareted_per_scope.pop().unwrap();
    for (sym, span) in cur_scope_vars {
      let status = self.symbols.get_mut(&sym).unwrap().pop().unwrap();
      if status.should_report() {
        self.report(sym, span);
      }
    }
  }
}

impl Visit for PreferConstVisitor {
  noop_visit_type!();

  fn visit_module(&mut self, module: &Module, _parent: &dyn Node) {
    self.enter_scope();
    module.visit_children_with(self);
    self.exit_scope();
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _parent: &dyn Node) {
    self.enter_scope();
    block_stmt.visit_children_with(self);
    self.exit_scope();
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    var_decl.visit_children_with(self);
    if var_decl.kind != VarDeclKind::Let {
      return;
    }

    for decl in &var_decl.decls {
      self.extract_decl_idents(&decl.name, decl.init.is_some());
    }
  }

  fn visit_assign_expr(
    &mut self,
    assign_expr: &AssignExpr,
    _parent: &dyn Node,
  ) {
    assign_expr.visit_children_with(self);
    match &assign_expr.left {
      PatOrExpr::Pat(pat) => self.extract_assign_idents(&**pat),
      PatOrExpr::Expr(_) => {}
    };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn hoge() {
    assert_lint_ok::<PreferConst>(r#"let x; { x = 0; } foo(x);"#);
  }

  #[test]
  fn prefer_const_valid() {
    assert_lint_ok_n::<PreferConst>(vec![
      r#"var x = 0;"#,
      r#"let x;"#,
      r#"let x; { x = 0; } foo(x);"#,
      r#"let x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"for (let i = 0, end = 10; i < end; ++i) {}"#,
      r#"for (let i in [1,2,3]) { i = 0; }"#,
      r#"for (let x of [1,2,3]) { x = 0; }"#,
      r#"(function() { var x = 0; })();"#,
      r#"(function() { let x; })();"#,
      r#"(function() { let x; { x = 0; } foo(x); })();"#,
      r#"(function() { let x = 0; x = 1; })();"#,
      r#"(function() { const x = 0; })();"#,
      r#"(function() { for (let i = 0, end = 10; i < end; ++i) {} })();"#,
      r#"(function() { for (let i in [1,2,3]) { i = 0; } })();"#,
      r#"(function() { for (let x of [1,2,3]) { x = 0; } })();"#,
      r#"(function(x = 0) { })();"#,
      r#"let a; while (a = foo());"#,
      r#"let a; do {} while (a = foo());"#,
      r#"let a; for (; a = foo(); );"#,
      r#"let a; for (;; ++a);"#,
      r#"let a; for (const {b = ++a} in foo());"#,
      r#"let a; for (const {b = ++a} of foo());"#,
      r#"let a; for (const x of [1,2,3]) { if (a) {} a = foo(); }"#,
      r#"let a; for (const x of [1,2,3]) { a = a || foo(); bar(a); }"#,
      r#"let a; for (const x of [1,2,3]) { foo(++a); }"#,
      r#"let a; function foo() { if (a) {} a = bar(); }"#,
      r#"let a; function foo() { a = a || bar(); baz(a); }"#,
      r#"let a; function foo() { bar(++a); }"#,
      r#"
    let id;
    function foo() {
        if (typeof id !== 'undefined') {
            return;
        }
        id = setInterval(() => {}, 250);
    }
    foo();
  "#,
      r#"/*exported a*/ let a; function init() { a = foo(); }"#,
      r#"/*exported a*/ let a = 1"#,
      r#"let a; if (true) a = 0; foo(a);"#,
      r#"
        (function (a) {
            let b;
            ({ a, b } = obj);
        })();
        "#,
      r#"
        (function (a) {
            let b;
            ([ a, b ] = obj);
        })();
        "#,
      r#"var a; { var b; ({ a, b } = obj); }"#,
      r#"let a; { let b; ({ a, b } = obj); }"#,
      r#"var a; { var b; ([ a, b ] = obj); }"#,
      r#"let a; { let b; ([ a, b ] = obj); }"#,
      r#"let x; { x = 0; foo(x); }"#,
      r#"(function() { let x; { x = 0; foo(x); } })();"#,
      r#"let x; for (const a of [1,2,3]) { x = foo(); bar(x); }"#,
      r#"(function() { let x; for (const a of [1,2,3]) { x = foo(); bar(x); } })();"#,
      r#"let x; for (x of array) { x; }"#,
      r#"let predicate; [typeNode.returnType, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType, ...predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType,, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType=5, predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType=5], predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [typeNode.returnType, [predicate]] = foo();"#,
      r#"let predicate; [, [typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, predicate}] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, ...predicate}] = foo();"#,
      r#"let a; const b = {}; ({ a, c: b.c } = func());"#,
      r#"const x = [1,2]; let y; [,y] = x; y = 0;"#,
      r#"const x = [1,2,3]; let y, z; [y,,z] = x; y = 0; z = 0;"#,
    ]);
  }

  #[test]
  fn prefer_const_invalid() {
    assert_lint_err::<PreferConst>(r#"let x = 1; foo(x);"#, 0);
    assert_lint_err::<PreferConst>(r#"for (let i in [1,2,3]) { foo(i); }"#, 0);
    assert_lint_err::<PreferConst>(r#"for (let x of [1,2,3]) { foo(x); }"#, 0);
    assert_lint_err::<PreferConst>(r#"let [x = -1, y] = [1,2]; y = 0;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let {a: x = -1, b: y} = {a:1,b:2}; y = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let x = 1; foo(x); })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let i in [1,2,3]) { foo(i); } })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let x of [1,2,3]) { foo(x); } })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let [x = -1, y] = [1,2]; y = 0; })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let f = (function() { let g = x; })(); f = 1;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let {a: x = -1, b: y} = {a:1,b:2}; y = 0; })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x = 0; { let x = 1; foo(x); } x = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"for (let i = 0; i < 10; ++i) { let x = 1; foo(x); }"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"for (let i in [1,2,3]) { let x = 1; foo(x); }"#,
      0,
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       a = 1;
   }
};
    "#,
      0,
      0,
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       ({a} = 1);
   }
};
    "#,
      0,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let x; x = 0;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"switch (a) { case 0: let x; x = 0; }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"(function() { let x; x = 1; })();"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let {a = 0, b} = obj; b = 0; foo(a, b);"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let {a: {b, c}} = {a: {b: 1, c: 2}}; b = 3;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let a, b; ({a = 0, b} = obj); b = 0; foo(a, b);"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let [a] = [1]"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a} = obj"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a = 0, b} = obj, c = a; b = a;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x; function foo() { bar(x); } x = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"/*eslint use-x:error*/ let x = 1"#, 0);
    assert_lint_err::<PreferConst>(
      r#"/*eslint use-x:error*/ { let x = 1 }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let { foo, bar } = baz;"#, 0);
    assert_lint_err::<PreferConst>(r#"const x = [1,2]; let [,y] = x;"#, 0);
    assert_lint_err::<PreferConst>(r#"const x = [1,2,3]; let [y,,z] = x;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}, ...bar ] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, ...predicate} ] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let x = 'x', y = 'y';"#, 0);
    assert_lint_err::<PreferConst>(r#"let x = 'x', y = 'y'; x = 1"#, 0);
    assert_lint_err::<PreferConst>(r#"let x = 1, y = 'y'; let z = 1;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let { a, b, c} = obj; let { x, y, z} = anotherObj; x = 2;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x = 'x', y = 'y'; function someFunc() { let a = 1, b = 2; foo(a, b) }"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let someFunc = () => { let a = 1, b = 2; foo(a, b) }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let {a, b} = c, d;"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a, b, c} = {}, e, f;"#, 0);
    assert_lint_err_on_line::<PreferConst>(
      r#"
function a() {
  let foo = 0,
  bar = 1;
  foo = 1;
}
function b() {
  let foo = 0,
  bar = 2;
  foo = 2;
}
    "#,
      0,
      0,
    );
  }
}
