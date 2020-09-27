// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::HashMap;
use std::sync::Arc;
use swc_atoms::JsWord;
use swc_common::Span;
use swc_ecmascript::ast::{
  ArrowExpr, AssignExpr, BlockStmt, CatchClause, DoWhileStmt, Expr, ForInStmt,
  ForOfStmt, ForStmt, Function, Ident, IfStmt, Module, ObjectPatProp, Pat,
  PatOrExpr, UpdateExpr, VarDecl, VarDeclKind, VarDeclOrExpr, VarDeclOrPat,
  WhileStmt, WithStmt,
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
  in_for_init: bool,
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

  fn insert_var(&mut self, ident: &Ident, has_init: bool, in_for_init: bool) {
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
        in_for_init,
      });
  }

  // Returns `Option<()>` to use question operator
  fn mark_reassigned(
    &mut self,
    ident: &Ident,
    force_reassigned: bool,
  ) -> Option<()> {
    // If this ident is not registered, do nothing.
    // (Most likely this happens if this ident is declared as a function parameter.)
    let status = self.symbols.get_mut(&ident.sym)?.last_mut()?;

    let declared_in_cur_scope = self
      .vars_declareted_per_scope
      .last()?
      .contains_key(&ident.sym);
    use Initalized::*;
    match status.initialized {
      NotYet => {
        status.initialized = if declared_in_cur_scope {
          SameScope
        } else {
          DifferentScope
        };
        if force_reassigned {
          status.reassigned = true;
        }
      }
      _ => {
        status.reassigned = true;
      }
    }

    None
  }

  fn extract_decl_idents(
    &mut self,
    pat: &Pat,
    has_init: bool,
    in_for_init: bool,
  ) {
    match pat {
      Pat::Ident(ident) => self.insert_var(ident, has_init, in_for_init),
      Pat::Array(array_pat) => {
        for elem in &array_pat.elems {
          if let Some(elem_pat) = elem {
            self.extract_decl_idents(elem_pat, has_init, in_for_init);
          }
        }
      }
      Pat::Rest(rest_pat) => {
        self.extract_decl_idents(&*rest_pat.arg, has_init, in_for_init)
      }
      Pat::Object(object_pat) => {
        for prop in &object_pat.props {
          match prop {
            ObjectPatProp::KeyValue(key_value) => {
              self.extract_decl_idents(&*key_value.value, has_init, in_for_init)
            }
            ObjectPatProp::Assign(assign) => {
              if assign.value.is_some() {
                self.insert_var(&assign.key, true, in_for_init);
              } else {
                self.insert_var(&assign.key, has_init, in_for_init);
              }
            }
            ObjectPatProp::Rest(rest) => {
              self.extract_decl_idents(&*rest.arg, has_init, in_for_init)
            }
          }
        }
      }
      Pat::Assign(assign_pat) => {
        self.extract_decl_idents(&*assign_pat.left, true, in_for_init)
      }
      _ => {}
    }
  }

  fn extract_assign_idents(&mut self, pat: &Pat) {
    fn extract_idents_rec<'a, 'b>(
      pat: &'a Pat,
      idents: &'b mut Vec<&'a Ident>,
      num_args: &'b mut usize,
    ) {
      match pat {
        Pat::Ident(ident) => {
          *num_args += 1;
          idents.push(ident);
        }
        Pat::Array(array_pat) => {
          for elem in &array_pat.elems {
            if let Some(elem_pat) = elem {
              extract_idents_rec(elem_pat, idents, num_args);
            }
          }
        }
        Pat::Rest(rest_pat) => {
          extract_idents_rec(&*rest_pat.arg, idents, num_args)
        }
        Pat::Object(object_pat) => {
          for prop in &object_pat.props {
            match prop {
              ObjectPatProp::KeyValue(key_value) => {
                extract_idents_rec(&*key_value.value, idents, num_args);
              }
              ObjectPatProp::Assign(assign) => {
                *num_args += 1;
                idents.push(&assign.key);
              }
              ObjectPatProp::Rest(rest) => {
                extract_idents_rec(&*rest.arg, idents, num_args)
              }
            }
          }
        }
        Pat::Assign(assign_pat) => {
          extract_idents_rec(&*assign_pat.left, idents, num_args)
        }
        Pat::Expr(_) => {
          *num_args += 1;
        }
        _ => {}
      }
    }

    let mut idents = Vec::new();
    let mut num_args = 0;
    extract_idents_rec(pat, &mut idents, &mut num_args);

    // If this pat contains two or more arguments, then all the idents should be marked as "reassigned"
    // so that we will not report them as error. This is bacause they couldn't be separately declared
    // as `const`.
    let force_reassigned = num_args >= 2;

    for ident in idents {
      self.mark_reassigned(ident, force_reassigned);
    }
  }

  fn enter_scope(&mut self) {
    self.vars_declareted_per_scope.push(HashMap::new());
  }

  fn exit_scope(&mut self) {
    let cur_scope_vars = self.vars_declareted_per_scope.pop().unwrap();
    let mut for_init_vars = Vec::new();
    for (sym, span) in cur_scope_vars {
      let status = self.symbols.get_mut(&sym).unwrap().pop().unwrap();
      if status.in_for_init {
        for_init_vars.push((sym, span, status));
      } else if status.should_report() {
        self.report(sym, span);
      }
    }

    // With regard to init sections of for statements, we should report diagnostics only if *all*
    // variables there need to be reported.
    if for_init_vars.iter().all(|v| v.2.should_report()) {
      for (sym, span, _) in for_init_vars {
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

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _parent: &dyn Node) {
    self.enter_scope();

    if_stmt.test.visit_children_with(self);
    if_stmt.cons.visit_children_with(self);

    self.exit_scope();

    if let Some(alt) = &if_stmt.alt {
      self.enter_scope();
      alt.visit_children_with(self);
      self.exit_scope();
    }
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _parent: &dyn Node) {
    self.enter_scope();

    match &for_stmt.init {
      Some(VarDeclOrExpr::VarDecl(var_decl)) => {
        var_decl.visit_children_with(self);
        if var_decl.kind == VarDeclKind::Let {
          for decl in &var_decl.decls {
            self.extract_decl_idents(&decl.name, decl.init.is_some(), true);
          }
        }
      }
      Some(VarDeclOrExpr::Expr(expr)) => {
        expr.visit_children_with(self);
      }
      None => {}
    }

    if let Some(test_expr) = &for_stmt.test {
      test_expr.visit_children_with(self);
    }
    if let Some(update_expr) = &for_stmt.update {
      update_expr.visit_children_with(self);
    }
    for_stmt.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, _parent: &dyn Node) {
    self.enter_scope();

    while_stmt.test.visit_children_with(self);
    while_stmt.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &DoWhileStmt,
    _parent: &dyn Node,
  ) {
    self.enter_scope();

    do_while_stmt.body.visit_children_with(self);
    do_while_stmt.test.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _parent: &dyn Node) {
    self.enter_scope();

    match &for_of_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        if var_decl.kind == VarDeclKind::Let {
          for decl in &var_decl.decls {
            self.extract_decl_idents(&decl.name, true, false);
          }
        }
      }
      VarDeclOrPat::Pat(pat) => {
        self.extract_assign_idents(pat);
      }
    }
    for_of_stmt.right.visit_children_with(self);
    for_of_stmt.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, _parent: &dyn Node) {
    self.enter_scope();

    match &for_in_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        if var_decl.kind == VarDeclKind::Let {
          for decl in &var_decl.decls {
            self.extract_decl_idents(&decl.name, true, false);
          }
        }
      }
      VarDeclOrPat::Pat(pat) => {
        self.extract_assign_idents(pat);
      }
    }
    for_in_stmt.right.visit_children_with(self);
    for_in_stmt.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.enter_scope();

    arrow_expr.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.enter_scope();

    if let Some(body) = &function.body {
      body.visit_children_with(self);
    }

    self.exit_scope();
  }

  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _parent: &dyn Node) {
    self.enter_scope();

    with_stmt.obj.visit_children_with(self);
    with_stmt.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_catch_clause(
    &mut self,
    catch_clause: &CatchClause,
    _parent: &dyn Node,
  ) {
    self.enter_scope();

    if let Some(param) = &catch_clause.param {
      self.extract_decl_idents(param, true, false);
    }
    catch_clause.body.visit_children_with(self);

    self.exit_scope();
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    var_decl.visit_children_with(self);
    if var_decl.kind != VarDeclKind::Let {
      return;
    }

    for decl in &var_decl.decls {
      self.extract_decl_idents(&decl.name, decl.init.is_some(), false);
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

  fn visit_update_expr(
    &mut self,
    update_expr: &UpdateExpr,
    _parent: &dyn Node,
  ) {
    match &*update_expr.arg {
      Expr::Ident(ident) => {
        self.mark_reassigned(ident, false);
      }
      otherwise => otherwise.visit_children_with(self),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // TODO(magurotuan) remove this test
  #[test]
  fn hoge() {
    assert_lint_err::<PreferConst>(r#"for (let i in [1,2,3]) { foo(i); }"#, 9);
    assert_lint_err::<PreferConst>(r#"for (let x of [1,2,3]) { foo(x); }"#, 9);
  }

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/prefer-const.js
  // MIT Licensed.

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
      // TODO(magurotuna): this is ported from ESLint, but I have no idea why this is valid,
      // so comment it out for now.
      // r#"/*exported a*/ let a = 1"#,
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
    assert_lint_err::<PreferConst>(r#"let x = 1; foo(x);"#, 4);
    assert_lint_err::<PreferConst>(r#"for (let i in [1,2,3]) { foo(i); }"#, 9);
    assert_lint_err::<PreferConst>(r#"for (let x of [1,2,3]) { foo(x); }"#, 9);
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
