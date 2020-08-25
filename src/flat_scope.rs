use std::collections::HashMap;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, CatchClause, ClassDecl, DoWhileStmt,
  FnDecl, ForInStmt, ForOfStmt, ForStmt, Function, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier, Invalid,
  Module, Param, Pat, SwitchStmt, VarDecl, VarDeclKind, WhileStmt, WithStmt,
};
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

#[derive(Debug)]
pub struct FlatScope {
  vars: HashMap<Id, Var>,
}

#[derive(Debug)]
pub struct Var {
  path: Vec<ScopeKind>,
  kind: BindingKind,
}

impl Var {
  /// Empty path means root scope.
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

pub fn analyze(module: &Module) -> FlatScope {
  let mut scope = FlatScope {
    vars: Default::default(),
  };
  let mut path = vec![];

  module.visit_with(
    &Invalid { span: DUMMY_SP },
    &mut Analyzer {
      scope: &mut scope,
      path: &mut path,
    },
  );

  scope
}

struct Analyzer<'a> {
  scope: &'a mut FlatScope,
  path: &'a mut Vec<ScopeKind>,
}

impl Analyzer<'_> {
  fn declare(&mut self, kind: BindingKind, i: &Ident) {
    self.scope.vars.insert(
      i.to_id(),
      Var {
        kind,
        path: self.path.clone(),
      },
    );
  }

  fn declare_pat(&mut self, kind: BindingKind, pat: &Pat) {
    let ids: Vec<Id> = find_ids(pat);
    let path = self.path.clone();

    self.scope.vars.extend(ids.into_iter().map(|id| {
      let var = Var {
        kind,
        path: path.clone(),
      };
      (id, var)
    }));
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
