// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use dprint_swc_ecma_ast_view as ast_view;
use std::collections::HashMap;
use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, CatchClause, ClassDecl, ClassExpr,
  DoWhileStmt, Expr, FnDecl, FnExpr, ForInStmt, ForOfStmt, ForStmt, Function,
  Ident, ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  Invalid, Param, Pat, SwitchStmt, VarDecl, VarDeclKind, WhileStmt, WithStmt,
};
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

#[derive(Debug, Default)]
pub struct Scope {
  vars: HashMap<Id, Var>,
  symbols: HashMap<JsWord, Vec<Id>>,
}

impl Scope {
  pub fn analyze(program: ast_view::Program) -> Self {
    let mut scope = Self::default();
    let mut path = vec![];

    match program {
      ast_view::Program::Module(module) => {
        module.inner.visit_with(
          &Invalid { span: DUMMY_SP },
          &mut Analyzer {
            scope: &mut scope,
            path: &mut path,
          },
        );
      }
      ast_view::Program::Script(script) => {
        script.inner.visit_with(
          &Invalid { span: DUMMY_SP },
          &mut Analyzer {
            scope: &mut scope,
            path: &mut path,
          },
        );
      }
    };

    scope
  }

  // Get all declarations with a symbol.
  pub fn ids_with_symbol(&self, sym: &JsWord) -> Option<&Vec<Id>> {
    self.symbols.get(sym)
  }

  pub fn var(&self, id: &Id) -> Option<&Var> {
    self.vars.get(id)
  }
}

#[derive(Debug)]
pub struct Var {
  path: Vec<ScopeKind>,
  kind: BindingKind,
}

impl Var {
  /// Empty path means root scope.
  #[allow(dead_code)]
  pub fn path(&self) -> &[ScopeKind] {
    &self.path
  }

  pub fn kind(&self) -> BindingKind {
    self.kind
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum BindingKind {
  Var,
  Const,
  Let,
  Function,
  Param,
  Class,
  CatchClause,
  Import,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ScopeKind {
  // Module,
  Arrow,
  Function,
  Block,
  Loop,
  Class,
  Switch,
  With,
  Catch,
}

struct Analyzer<'a> {
  scope: &'a mut Scope,
  path: &'a mut Vec<ScopeKind>,
}

impl Analyzer<'_> {
  fn declare_id(&mut self, kind: BindingKind, i: Id) {
    self.scope.vars.insert(
      i.clone(),
      Var {
        kind,
        path: self.path.clone(),
      },
    );
    self.scope.symbols.entry(i.0.clone()).or_default().push(i);
  }

  fn declare(&mut self, kind: BindingKind, i: &Ident) {
    self.declare_id(kind, i.to_id());
  }

  fn declare_pat(&mut self, kind: BindingKind, pat: &Pat) {
    let ids: Vec<Id> = find_ids(pat);

    for id in ids {
      self.declare_id(kind, id);
    }
  }

  fn visit_with_path<T>(&mut self, kind: ScopeKind, node: &T)
  where
    T: 'static + for<'any> VisitWith<Analyzer<'any>>,
  {
    self.path.push(kind);
    node.visit_with(node, self);
    self.path.pop();
  }

  fn with<F>(&mut self, kind: ScopeKind, op: F)
  where
    F: FnOnce(&mut Analyzer),
  {
    self.path.push(kind);
    op(self);
    self.path.pop();
  }
}

impl Visit for Analyzer<'_> {
  fn visit_arrow_expr(&mut self, n: &ArrowExpr, _: &dyn Node) {
    self.with(ScopeKind::Arrow, |a| n.visit_children_with(a))
  }

  /// Overriden not to add ScopeKind::Block
  fn visit_block_stmt_or_expr(&mut self, n: &BlockStmtOrExpr, _: &dyn Node) {
    match n {
      BlockStmtOrExpr::BlockStmt(s) => s.stmts.visit_with(n, self),
      BlockStmtOrExpr::Expr(e) => e.visit_with(n, self),
    }
  }

  fn visit_var_decl(&mut self, n: &VarDecl, _: &dyn Node) {
    n.decls.iter().for_each(|v| {
      v.init.visit_with(n, self);

      // If the class name and the variable name are the same like `let Foo = class Foo {}`,
      // this binding should be treated as `BindingKind::Class`.
      if let Some(expr) = &v.init {
        if let Expr::Class(ClassExpr {
          ident: Some(class_name),
          ..
        }) = &**expr
        {
          if let Pat::Ident(var_name) = &v.name {
            if var_name.id.sym == class_name.sym {
              self.declare(BindingKind::Class, class_name);
              return;
            }
          }
        }
      }

      self.declare_pat(
        match n.kind {
          VarDeclKind::Var => BindingKind::Var,
          VarDeclKind::Let => BindingKind::Let,
          VarDeclKind::Const => BindingKind::Const,
        },
        &v.name,
      );
    });
  }

  /// Overriden not to add ScopeKind::Block
  fn visit_function(&mut self, n: &Function, _: &dyn Node) {
    n.decorators.visit_with(n, self);
    n.params.visit_with(n, self);

    // Don't add ScopeKind::Block
    match &n.body {
      Some(s) => s.stmts.visit_with(n, self),
      None => {}
    }
  }

  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {
    self.declare(BindingKind::Function, &n.ident);

    self.visit_with_path(ScopeKind::Function, &n.function);
  }

  fn visit_fn_expr(&mut self, n: &FnExpr, _: &dyn Node) {
    if let Some(ident) = &n.ident {
      self.declare(BindingKind::Function, ident);
    }

    self.visit_with_path(ScopeKind::Function, &n.function);
  }

  fn visit_class_decl(&mut self, n: &ClassDecl, _: &dyn Node) {
    self.declare(BindingKind::Class, &n.ident);

    self.visit_with_path(ScopeKind::Class, &n.class);
  }

  fn visit_block_stmt(&mut self, n: &BlockStmt, _: &dyn Node) {
    self.visit_with_path(ScopeKind::Block, &n.stmts)
  }

  fn visit_catch_clause(&mut self, n: &CatchClause, _: &dyn Node) {
    if let Some(pat) = &n.param {
      self.declare_pat(BindingKind::CatchClause, pat);
    }
    self.visit_with_path(ScopeKind::Catch, &n.body)
  }

  fn visit_param(&mut self, n: &Param, _: &dyn Node) {
    self.declare_pat(BindingKind::Param, &n.pat);
  }

  fn visit_import_named_specifier(
    &mut self,
    n: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.declare(BindingKind::Import, &n.local);
  }

  fn visit_import_default_specifier(
    &mut self,
    n: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.declare(BindingKind::Import, &n.local);
  }

  fn visit_import_star_as_specifier(
    &mut self,
    n: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.declare(BindingKind::Import, &n.local);
  }

  fn visit_with_stmt(&mut self, n: &WithStmt, _: &dyn Node) {
    n.obj.visit_with(n, self);
    self.with(ScopeKind::With, |a| n.body.visit_children_with(a))
  }

  fn visit_for_stmt(&mut self, n: &ForStmt, _: &dyn Node) {
    n.init.visit_with(n, self);
    n.update.visit_with(n, self);
    n.test.visit_with(n, self);

    self.visit_with_path(ScopeKind::Loop, &n.body);
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    n.left.visit_with(n, self);
    n.right.visit_with(n, self);

    self.visit_with_path(ScopeKind::Loop, &n.body);
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    n.left.visit_with(n, self);
    n.right.visit_with(n, self);

    self.visit_with_path(ScopeKind::Loop, &n.body);
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.visit_with_path(ScopeKind::Loop, &n.body);
  }

  fn visit_while_stmt(&mut self, n: &WhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.visit_with_path(ScopeKind::Loop, &n.body);
  }

  fn visit_switch_stmt(&mut self, n: &SwitchStmt, _: &dyn Node) {
    n.discriminant.visit_with(n, self);

    self.visit_with_path(ScopeKind::Switch, &n.cases);
  }
}

#[cfg(test)]
mod tests {
  use super::{BindingKind, Scope, ScopeKind, Var};
  use crate::test_util;
  use swc_ecmascript::utils::Id;

  fn test_scope(source_code: &str, test: impl Fn(Scope)) {
    test_util::parse_and_then(source_code, |program| {
      let scope = Scope::analyze(program);
      test(scope);
    });
  }

  fn id(scope: &Scope, s: &str) -> Id {
    let ids = scope.ids_with_symbol(&s.into());
    if ids.is_none() {
      panic!("No identifier named {}", s);
    }
    let ids = ids.unwrap();
    if ids.len() > 1 {
      panic!("Multiple identifers named {} found", s);
    }

    ids.first().unwrap().clone()
  }

  fn var<'a>(scope: &'a Scope, symbol: &str) -> &'a Var {
    scope.var(&id(scope, symbol)).unwrap()
  }

  #[test]
  fn scopes() {
    let source_code = r#"
const a = "a";
const unused = "unused";
function asdf(b: number, c: string): number {
    console.log(a, b);
    {
      const c = 1;
      let d = 2;
    }
    return 1;
}
class Foo {
  #fizz = "fizz";
  bar() {
  }
}
try {
  // some code that might throw
  throw new Error("asdf");
} catch (e) {
  const msg = "asdf " + e.message;
}
"#;
    test_scope(source_code, |scope| {
      assert_eq!(var(&scope, "a").kind(), BindingKind::Const);
      assert_eq!(var(&scope, "a").path(), &[]);

      assert_eq!(var(&scope, "b").kind(), BindingKind::Param);
      assert_eq!(scope.ids_with_symbol(&"c".into()).unwrap().len(), 2);
      assert_eq!(
        var(&scope, "d").path(),
        &[ScopeKind::Function, ScopeKind::Block]
      );

      assert_eq!(var(&scope, "Foo").kind(), BindingKind::Class);
      assert_eq!(var(&scope, "Foo").path(), &[]);

      assert_eq!(var(&scope, "e").kind(), BindingKind::CatchClause);
      assert_eq!(var(&scope, "e").path(), &[]);
    });
  }
}
