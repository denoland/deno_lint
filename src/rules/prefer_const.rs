// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::BTreeMap;
use std::mem;
use std::sync::{Arc, Mutex};
use swc_atoms::JsWord;
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrowExpr, AssignExpr, BlockStmt, BlockStmtOrExpr, CatchClause, Class,
  Constructor, DoWhileStmt, Expr, ExprStmt, ForInStmt, ForOfStmt, ForStmt,
  Function, Ident, IfStmt, Module, ObjectPatProp, ParamOrTsParamProp, Pat,
  PatOrExpr, Stmt, TsParamPropParam, UpdateExpr, VarDecl, VarDeclKind,
  VarDeclOrExpr, VarDeclOrPat, WhileStmt, WithStmt,
};
use swc_ecmascript::utils::find_ids;
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
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = VariableCollector::new();
    collector.visit_module(module, module);

    let mut visitor =
      PreferConstVisitor::new(context, mem::take(&mut collector.scopes));
    visitor.visit_module(module, module);
  }
}

#[derive(Debug, Clone, Copy)]
struct Variable {
  span: Span,
  initialized: bool,
  reassigned: bool,
  /// If this variable is declared in "init" section of a for statement, it stores `Some(span)` where
  /// `span` is the span of the for statement. Otherwise, it stores `None`.
  in_for_init: Option<Span>,
  is_param: bool,
}

impl Variable {
  fn update(&mut self, initialized: bool, reassigned: bool) {
    self.initialized = initialized;
    self.reassigned = reassigned;
  }
  fn should_report(&self) -> bool {
    if self.is_param {
      return false;
    }

    // (initialized, reassigned): [return value]
    //
    // - (false, false): false
    // - (true, false): true
    // - (false, true): true
    // - (true, true): false
    self.initialized != self.reassigned
  }
}

type Scope = Arc<Mutex<RawScope>>;

#[derive(Debug)]
struct RawScope {
  parent: Option<Scope>,
  variables: BTreeMap<JsWord, Variable>,
}

impl RawScope {
  fn new(parent: Option<Scope>) -> Self {
    Self {
      parent,
      variables: BTreeMap::new(),
    }
  }
}

/// Looks for the variable status of the given ident by traversing from the current scope to the parent,
/// and updates its status.
fn update_variable_status(scope: Scope, ident: &Ident, force_reassigned: bool) {
  let mut cur_scope = Some(scope);
  while let Some(cur) = cur_scope {
    let mut lock = cur.lock().unwrap();
    if let Some(var) = lock.variables.get_mut(&ident.sym) {
      let (initialized, mut reassigned) = if var.initialized {
        (true, true)
      } else {
        (true, false)
      };

      reassigned |= force_reassigned;

      var.update(initialized, reassigned);
      return;
    }
    cur_scope = lock.parent.as_ref().map(Arc::clone);
  }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum ScopeRange {
  Global,
  Block(Span),
}

#[derive(Debug)]
struct VariableCollector {
  scopes: BTreeMap<ScopeRange, Scope>,
  cur_scope: ScopeRange,
}

impl VariableCollector {
  fn new() -> Self {
    Self {
      scopes: BTreeMap::new(),
      cur_scope: ScopeRange::Global,
    }
  }

  fn insert_var(
    &mut self,
    ident: &Ident,
    has_init: bool,
    in_for_init: Option<Span>,
    is_param: bool,
  ) {
    let mut scope = self.scopes.get(&self.cur_scope).unwrap().lock().unwrap();
    scope.variables.insert(
      ident.sym.clone(),
      Variable {
        span: ident.span,
        initialized: has_init,
        reassigned: false,
        in_for_init,
        is_param,
      },
    );
  }

  fn extract_decl_idents(
    &mut self,
    pat: &Pat,
    has_init: bool,
    in_for_init: Option<Span>,
  ) {
    match pat {
      Pat::Ident(ident) => self.insert_var(ident, has_init, in_for_init, false),
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
                self.insert_var(&assign.key, true, in_for_init, false);
              } else {
                self.insert_var(&assign.key, has_init, in_for_init, false);
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

  fn with_child_scope<F, S>(&mut self, node: S, op: F)
  where
    S: Spanned,
    F: FnOnce(&mut VariableCollector),
  {
    let parent_scope_range = self.cur_scope;
    let parent_scope = self.scopes.get(&parent_scope_range).map(Arc::clone);
    let child_scope = RawScope::new(parent_scope);
    self.scopes.insert(
      ScopeRange::Block(node.span()),
      Arc::new(Mutex::new(child_scope)),
    );
    self.cur_scope = ScopeRange::Block(node.span());
    op(self);
    self.cur_scope = parent_scope_range;
  }
}

impl Visit for VariableCollector {
  noop_visit_type!();

  fn visit_module(&mut self, module: &Module, _: &dyn Node) {
    let scope = RawScope::new(None);
    self
      .scopes
      .insert(ScopeRange::Global, Arc::new(Mutex::new(scope)));
    module.visit_children_with(self);
  }

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    self.with_child_scope(function, |a| {
      for param in &function.params {
        param.visit_children_with(a);
        let idents: Vec<Ident> = find_ids(&param.pat);
        for ident in idents {
          a.insert_var(&ident, true, None, true);
        }
      }
      if let Some(body) = &function.body {
        body.visit_children_with(a);
      }
    });
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    self.with_child_scope(arrow_expr, |a| {
      for param in &arrow_expr.params {
        param.visit_children_with(a);
        let idents: Vec<Ident> = find_ids(param);
        for ident in idents {
          a.insert_var(&ident, true, None, true);
        }
      }
      match &arrow_expr.body {
        BlockStmtOrExpr::BlockStmt(block_stmt) => {
          block_stmt.visit_children_with(a);
        }
        BlockStmtOrExpr::Expr(expr) => {
          expr.visit_children_with(a);
        }
      }
    });
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _: &dyn Node) {
    self.with_child_scope(block_stmt, |a| {
      block_stmt.visit_children_with(a);
    });
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _: &dyn Node) {
    self.with_child_scope(for_stmt, |a| {
      match &for_stmt.init {
        Some(VarDeclOrExpr::VarDecl(var_decl)) => {
          var_decl.visit_children_with(a);
          if var_decl.kind == VarDeclKind::Let {
            for decl in &var_decl.decls {
              a.extract_decl_idents(
                &decl.name,
                decl.init.is_some(),
                Some(for_stmt.span),
              );
            }
          }
        }
        Some(VarDeclOrExpr::Expr(expr)) => {
          expr.visit_children_with(a);
        }
        None => {}
      }

      if let Some(test_expr) = &for_stmt.test {
        test_expr.visit_children_with(a);
      }
      if let Some(update_expr) = &for_stmt.update {
        update_expr.visit_children_with(a);
      }

      if let Stmt::Block(block_stmt) = &*for_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _: &dyn Node) {
    self.with_child_scope(for_of_stmt, |a| {
      if let VarDeclOrPat::VarDecl(var_decl) = &for_of_stmt.left {
        if var_decl.kind == VarDeclKind::Let {
          for decl in &var_decl.decls {
            a.extract_decl_idents(&decl.name, true, None);
          }
        }
      }

      for_of_stmt.right.visit_children_with(a);

      if let Stmt::Block(block_stmt) = &*for_of_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_of_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, _: &dyn Node) {
    self.with_child_scope(for_in_stmt, |a| {
      if let VarDeclOrPat::VarDecl(var_decl) = &for_in_stmt.left {
        if var_decl.kind == VarDeclKind::Let {
          for decl in &var_decl.decls {
            a.extract_decl_idents(&decl.name, true, None);
          }
        }
      }

      for_in_stmt.right.visit_children_with(a);

      if let Stmt::Block(block_stmt) = &*for_in_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_in_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _: &dyn Node) {
    self.with_child_scope(if_stmt, |a| {
      if_stmt.test.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*if_stmt.cons {
        body.visit_children_with(a);
      } else {
        if_stmt.cons.visit_children_with(a);
      }
    });

    if let Some(alt) = &if_stmt.alt {
      self.with_child_scope(alt, |a| {
        alt.visit_children_with(a);
      });
    }
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, _: &dyn Node) {
    self.with_child_scope(while_stmt, |a| {
      while_stmt.test.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*while_stmt.body {
        body.visit_children_with(a);
      } else {
        while_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt, _: &dyn Node) {
    self.with_child_scope(do_while_stmt, |a| {
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*do_while_stmt.body {
        body.visit_children_with(a);
      } else {
        do_while_stmt.body.visit_children_with(a);
      }
      do_while_stmt.test.visit_children_with(a);
    });
  }

  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _: &dyn Node) {
    self.with_child_scope(with_stmt, |a| {
      with_stmt.obj.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*with_stmt.body {
        body.visit_children_with(a);
      } else {
        with_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause, _: &dyn Node) {
    self.with_child_scope(catch_clause, |a| {
      if let Some(param) = &catch_clause.param {
        let idents: Vec<Ident> = find_ids(param);
        for ident in idents {
          a.insert_var(&ident, true, None, true);
        }
      }
      catch_clause.body.visit_children_with(a);
    });
  }

  fn visit_class(&mut self, class: &Class, _: &dyn Node) {
    for decorator in &class.decorators {
      decorator.visit_children_with(self);
    }
    if let Some(super_class) = &class.super_class {
      super_class.visit_children_with(self);
    }
    self.with_child_scope(class, |a| {
      for member in &class.body {
        member.visit_children_with(a);
      }
    });
  }

  fn visit_constructor(&mut self, constructor: &Constructor, _: &dyn Node) {
    self.with_child_scope(constructor, |a| {
      for param in &constructor.params {
        match param {
          ParamOrTsParamProp::TsParamProp(ts_param_prop) => {
            for decorator in &ts_param_prop.decorators {
              decorator.visit_children_with(a);
            }
            match &ts_param_prop.param {
              TsParamPropParam::Ident(ident) => {
                a.insert_var(ident, true, None, true);
              }
              TsParamPropParam::Assign(assign_pat) => {
                assign_pat.visit_children_with(a);
                let idents: Vec<Ident> = find_ids(&assign_pat.left);
                for ident in idents {
                  a.insert_var(&ident, true, None, true);
                }
              }
            }
          }
          ParamOrTsParamProp::Param(param) => {
            param.visit_children_with(a);
            let idents: Vec<Ident> = find_ids(&param.pat);
            for ident in idents {
              a.insert_var(&ident, true, None, true);
            }
          }
        }
      }

      if let Some(body) = &constructor.body {
        body.visit_children_with(a);
      }
    });
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _: &dyn Node) {
    var_decl.visit_children_with(self);
    if var_decl.kind == VarDeclKind::Let {
      for decl in &var_decl.decls {
        self.extract_decl_idents(&decl.name, decl.init.is_some(), None);
      }
    }
  }
}

struct PreferConstVisitor<'c> {
  scopes: BTreeMap<ScopeRange, Scope>,
  cur_scope: ScopeRange,
  context: &'c mut Context,
}

impl<'c> PreferConstVisitor<'c> {
  fn new(
    context: &'c mut Context,
    scopes: BTreeMap<ScopeRange, Scope>,
  ) -> Self {
    Self {
      context,
      scopes,
      cur_scope: ScopeRange::Global,
    }
  }

  fn report(&mut self, sym: &JsWord, span: Span) {
    self.context.add_diagnostic(
      span,
      "prefer-const",
      &format!(
        "'{}' is never reassigned. Use 'const' instead",
        sym.to_string()
      ),
    );
  }

  fn with_child_scope<F, S>(&mut self, node: &S, op: F)
  where
    S: Spanned,
    F: FnOnce(&mut Self),
  {
    let parent_scope_range = self.cur_scope;
    self.cur_scope = ScopeRange::Block(node.span());
    op(self);
    self.cur_scope = parent_scope_range;
  }

  fn mark_reassigned(&mut self, ident: &Ident, force_reassigned: bool) {
    let scope = self.scopes.get(&self.cur_scope).unwrap();
    update_variable_status(Arc::clone(scope), ident, force_reassigned);
  }

  fn extract_assign_idents(&mut self, pat: &Pat) {
    fn extract_idents_rec<'a, 'b>(
      pat: &'a Pat,
      idents: &'b mut Vec<&'a Ident>,
      has_member_expr: &'b mut bool,
    ) {
      match pat {
        Pat::Ident(ident) => {
          idents.push(ident);
        }
        Pat::Array(array_pat) => {
          for elem in &array_pat.elems {
            if let Some(elem_pat) = elem {
              extract_idents_rec(elem_pat, idents, has_member_expr);
            }
          }
        }
        Pat::Rest(rest_pat) => {
          extract_idents_rec(&*rest_pat.arg, idents, has_member_expr)
        }
        Pat::Object(object_pat) => {
          for prop in &object_pat.props {
            match prop {
              ObjectPatProp::KeyValue(key_value) => {
                extract_idents_rec(&*key_value.value, idents, has_member_expr);
              }
              ObjectPatProp::Assign(assign) => {
                idents.push(&assign.key);
              }
              ObjectPatProp::Rest(rest) => {
                extract_idents_rec(&*rest.arg, idents, has_member_expr)
              }
            }
          }
        }
        Pat::Assign(assign_pat) => {
          extract_idents_rec(&*assign_pat.left, idents, has_member_expr)
        }
        Pat::Expr(expr) => {
          if let Expr::Member(_) = &**expr {
            *has_member_expr = true;
          }
        }
        _ => {}
      }
    }

    let mut idents = Vec::new();
    let mut has_member_expr = false;
    extract_idents_rec(pat, &mut idents, &mut has_member_expr);

    let has_outer_scope_or_param_var = idents
      .iter()
      .any(|i| self.declared_outer_scope_or_param_var(i));

    for ident in idents {
      // If tha pat contains either of the following:
      //
      // - MemberExpresion
      // - variable declared in outer scope
      // - variable that is a function parameter
      //
      // then all the idents should be marked as "reassigned" so that we will not report them as errors,
      // bacause in this case they couldn't be separately declared as `const`.
      self.mark_reassigned(
        ident,
        has_member_expr || has_outer_scope_or_param_var,
      );
    }
  }

  /// Checks if this ident has its declaration in outer scope or in function parameter.
  fn declared_outer_scope_or_param_var(&self, ident: &Ident) -> bool {
    let mut cur_scope = self.scopes.get(&self.cur_scope).map(Arc::clone);
    let mut is_first_loop = true;
    while let Some(cur) = cur_scope {
      let lock = cur.lock().unwrap();
      if let Some(var) = lock.variables.get(&ident.sym) {
        if is_first_loop {
          return var.is_param;
        } else {
          return true;
        }
      }
      is_first_loop = false;
      cur_scope = lock.parent.as_ref().map(Arc::clone);
    }
    // If the declaration isn't found, most likely it means the ident is declared with `var`
    false
  }

  fn exit_module(&mut self) {
    let mut for_init_vars = BTreeMap::new();
    let scopes = self.scopes.clone();
    for scope in scopes.values() {
      for (sym, status) in scope.lock().unwrap().variables.iter() {
        if let Some(for_span) = status.in_for_init {
          for_init_vars
            .entry(for_span)
            .or_insert_with(Vec::new)
            .push((sym.clone(), *status));
        } else if status.should_report() {
          self.report(sym, status.span);
        }
      }
    }

    // With regard to init sections of for statements, we should report diagnostics only if *all*
    // variables there need to be reported.
    for (sym, var) in for_init_vars
      .iter()
      .filter_map(|(_, vars)| {
        if vars.iter().all(|(_, status)| status.should_report()) {
          Some(vars)
        } else {
          None
        }
      })
      .flatten()
    {
      self.report(sym, var.span);
    }
  }
}

impl<'c> Visit for PreferConstVisitor<'c> {
  noop_visit_type!();

  fn visit_module(&mut self, module: &Module, _: &dyn Node) {
    module.visit_children_with(self);
    self.exit_module();
  }

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _: &dyn Node) {
    // This only handles _nested_ `AssignmentExpression` since not nested `AssignExpression` (i.e. the direct child of
    // `ExpressionStatement`) is already handled by `visit_expr_stmt`. The variables within nested
    // `AssignmentExpression` should be marked as "reassigned" even if it's not been yet initialized, otherwise it
    // would result in false positives.
    // See https://github.com/denoland/deno_lint/issues/358
    assign_expr.visit_children_with(self);

    let idents: Vec<Ident> = match &assign_expr.left {
      PatOrExpr::Pat(pat) => find_ids(pat), // find_ids doesn't work for Expression
      PatOrExpr::Expr(expr) if expr.is_ident() => {
        let ident = (**expr).clone().expect_ident();
        vec![ident]
      }
      _ => vec![],
    };

    for ident in idents {
      self.mark_reassigned(&ident, true);
    }
  }

  fn visit_expr_stmt(&mut self, expr_stmt: &ExprStmt, _: &dyn Node) {
    let mut expr = &*expr_stmt.expr;

    // Unwrap parentheses
    while let Expr::Paren(e) = expr {
      expr = &*e.expr;
    }

    match expr {
      Expr::Assign(assign_expr) => {
        match &assign_expr.left {
          PatOrExpr::Pat(pat) => self.extract_assign_idents(&**pat),
          PatOrExpr::Expr(expr) => match &**expr {
            Expr::Ident(ident) => {
              self.mark_reassigned(ident, false);
            }
            otherwise => {
              otherwise.visit_children_with(self);
            }
          },
        };
        assign_expr.visit_children_with(self);
      }
      _ => expr_stmt.visit_children_with(self),
    }
  }

  fn visit_update_expr(&mut self, update_expr: &UpdateExpr, _: &dyn Node) {
    match &*update_expr.arg {
      Expr::Ident(ident) => {
        self.mark_reassigned(
          ident,
          self.declared_outer_scope_or_param_var(ident),
        );
      }
      otherwise => otherwise.visit_children_with(self),
    }
  }

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    self.with_child_scope(function, |a| {
      if let Some(body) = &function.body {
        body.visit_children_with(a);
      }
    });
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    self.with_child_scope(arrow_expr, |a| match &arrow_expr.body {
      BlockStmtOrExpr::BlockStmt(block_stmt) => {
        block_stmt.visit_children_with(a);
      }
      BlockStmtOrExpr::Expr(expr) => {
        expr.visit_children_with(a);
      }
    });
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _: &dyn Node) {
    self.with_child_scope(block_stmt, |a| block_stmt.visit_children_with(a));
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _: &dyn Node) {
    self.with_child_scope(for_stmt, |a| {
      for_stmt.init.visit_children_with(a);
      for_stmt.test.visit_children_with(a);
      for_stmt.update.visit_children_with(a);

      if let Stmt::Block(block_stmt) = &*for_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _: &dyn Node) {
    self.with_child_scope(for_of_stmt, |a| {
      match &for_of_stmt.left {
        VarDeclOrPat::VarDecl(var_decl) => {
          var_decl.visit_with(&for_of_stmt.left, a);
        }
        VarDeclOrPat::Pat(pat) => {
          a.extract_assign_idents(pat);
        }
      }

      for_of_stmt.right.visit_children_with(a);

      if let Stmt::Block(block_stmt) = &*for_of_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_of_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, _: &dyn Node) {
    self.with_child_scope(for_in_stmt, |a| {
      match &for_in_stmt.left {
        VarDeclOrPat::VarDecl(var_decl) => {
          var_decl.visit_with(&for_in_stmt.left, a);
        }
        VarDeclOrPat::Pat(pat) => {
          a.extract_assign_idents(pat);
        }
      }

      for_in_stmt.right.visit_children_with(a);

      if let Stmt::Block(block_stmt) = &*for_in_stmt.body {
        block_stmt.visit_children_with(a);
      } else {
        for_in_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _: &dyn Node) {
    self.with_child_scope(if_stmt, |a| {
      if_stmt.test.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*if_stmt.cons {
        body.visit_children_with(a);
      } else {
        if_stmt.cons.visit_children_with(a);
      }
    });

    if let Some(alt) = &if_stmt.alt {
      self.with_child_scope(alt, |a| {
        alt.visit_children_with(a);
      });
    }
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, _: &dyn Node) {
    self.with_child_scope(while_stmt, |a| {
      while_stmt.test.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*while_stmt.body {
        body.visit_children_with(a);
      } else {
        while_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt, _: &dyn Node) {
    self.with_child_scope(do_while_stmt, |a| {
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*do_while_stmt.body {
        body.visit_children_with(a);
      } else {
        do_while_stmt.body.visit_children_with(a);
      }
      do_while_stmt.test.visit_children_with(a);
    });
  }

  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _: &dyn Node) {
    self.with_child_scope(with_stmt, |a| {
      with_stmt.obj.visit_children_with(a);
      // BlockStmt needs special handling to avoid creating a duplicate scope
      if let Stmt::Block(body) = &*with_stmt.body {
        body.visit_children_with(a);
      } else {
        with_stmt.body.visit_children_with(a);
      }
    });
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause, _: &dyn Node) {
    self.with_child_scope(catch_clause, |a| {
      if let Some(param) = &catch_clause.param {
        param.visit_children_with(a);
      }
      catch_clause.body.visit_children_with(a);
    });
  }

  fn visit_class(&mut self, class: &Class, _: &dyn Node) {
    for decorator in &class.decorators {
      decorator.visit_children_with(self);
    }
    if let Some(super_class) = &class.super_class {
      super_class.visit_children_with(self);
    }
    self.with_child_scope(class, |a| {
      for member in &class.body {
        member.visit_children_with(a);
      }
    });
  }

  fn visit_constructor(&mut self, constructor: &Constructor, _: &dyn Node) {
    self.with_child_scope(constructor, |a| {
      for param in &constructor.params {
        param.visit_children_with(a);
      }

      if let Some(body) = &constructor.body {
        body.visit_children_with(a);
      }
    });
  }
}

#[cfg(test)]
mod variable_collector_tests {
  use super::*;
  use crate::test_util;

  fn collect(src: &str) -> VariableCollector {
    let module = test_util::parse(src);
    let mut v = VariableCollector::new();
    v.visit_module(&module, &module);
    v
  }

  fn variables(scope: &Scope) -> Vec<String> {
    scope
      .lock()
      .unwrap()
      .variables
      .keys()
      .map(|k| k.to_string())
      .collect()
  }

  #[test]
  fn collector_works_function() {
    let src = r#"
let global1;
function foo({ param1, key: param2 }) {
  let inner1 = 2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let foo_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "param1", "param2"], foo_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_arrow_function() {
    let src = r#"
let global1;
const arrow = (param1, ...param2) => {
  let inner1;
};
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let foo_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "param1", "param2"], foo_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_block() {
    let src = r#"
let global1;
{
  let inner1 = 1;
  let inner2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let inner_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "inner2"], inner_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_1() {
    let src = r#"
let global1;
if (true) {
  let inner1 = 1;
  let inner2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let inner_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "inner2"], inner_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_2() {
    let src = r#"
let global1;
if (true) {
  let cons = 1;
} else {
  let alt = 3;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons"], cons_vars);

    let alt_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["alt"], alt_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_3() {
    let src = r#"
let global1;
if (true) {
  let cons1 = 1;
} else if (false) {
  let cons2 = 2;
} else {
  let alt;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons1_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons1"], cons1_vars);

    let cons2_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons2"], cons2_vars);

    let alt_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["alt"], alt_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_4() {
    let src = r#"
let global1;
if (true) foo();
else if (false) bar();
else baz();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons1_vars = variables(scope_iter.next().unwrap());
    assert!(cons1_vars.is_empty());

    let cons2_vars = variables(scope_iter.next().unwrap());
    assert!(cons2_vars.is_empty());

    let alt_vars = variables(scope_iter.next().unwrap());
    assert!(alt_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_1() {
    let src = r#"
let global1;
for (let i = 0, j = 10; i < 10; i++) {
  let inner;
  j--;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_2() {
    let src = r#"
let global1;
for (let i = 0, j = 10; i < 10; i++) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_3() {
    let src = r#"
let global1 = 0;
for (global1 = 0; global1 < 10; ++global1) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_1() {
    let src = r#"
let global1;
for (let i of [1, 2, 3]) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_2() {
    let src = r#"
let global1;
for (let { i, j } of [{ i: 1, j: 2 }, { i : 3, j: 4 }]) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_3() {
    let src = r#"
let global1 = 0;
for (global1 of [1, 2, 3]) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_1() {
    let src = r#"
let global1;
for (let i in [1, 2, 3]) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_2() {
    let src = r#"
let global1;
for (let i in [1, 2, 3]) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_3() {
    let src = r#"
let global1 = 0;
for (global1 in [1, 2, 3]) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_while_1() {
    let src = r#"
let global1;
while (true) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], while_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_while_2() {
    let src = r#"
let global1;
while (true) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert!(while_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_do_while_1() {
    let src = r#"
let global1;
do {
  let inner;
} while (true)
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], while_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_do_while_2() {
    let src = r#"
let global1;
do foo(); while (true)
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert!(while_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_with_1() {
    let src = r#"
let global1;
with (foo) {
  let inner = 1;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let with_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], with_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_with_2() {
    let src = r#"
let global1;
with (foo) bar();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let with_vars = variables(scope_iter.next().unwrap());
    assert!(with_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_try_catch() {
    let src = r#"
let global1;
try {
  let tryVar;
  foo();
} catch(e) {
  let catchVar;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let try_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["tryVar"], try_vars);

    let catch_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["catchVar", "e"], catch_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_class() {
    let src = r#"
let global1;
class Foo {
  field;
  #privateField;
  constructor(consParam) {
    let cons;
  }
  get getter() {
    let g;
  }
  set setter() {
    let s;
  }
  static staticMethod(staticMethodParam) {
    let sm;
  }
  method(methodParam) {
    let m;
  }
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let class_vars = variables(scope_iter.next().unwrap());
    assert!(class_vars.is_empty()); // collector doesn't collect class fields

    let constructor_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons", "consParam"], constructor_vars);

    let getter_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["g"], getter_vars);

    let setter_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["s"], setter_vars);

    let static_method_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["sm", "staticMethodParam"], static_method_vars);

    let method_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["m", "methodParam"], method_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_complex_1() {
    let src = r#"
let global1;
function foo({ p1 = 0 }) {
  while (cond) {
    let while1 = true;
    if (while1) {
      break;
    }
    let [while2, { while3, key: while4 = 4 }] = bar();
  }
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let function_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["p1"], function_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["while1", "while2", "while3", "while4"], while_vars);

    let if_vars = variables(scope_iter.next().unwrap());
    assert!(if_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }
}

#[cfg(test)]
mod prefer_const_tests {
  use super::*;
  use crate::test_util::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/prefer-const.js
  // MIT Licensed.

  #[test]
  fn prefer_const_valid() {
    assert_lint_ok::<PreferConst>(r#"var x = 0;"#);
    assert_lint_ok::<PreferConst>(r#"let x;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; x += 1;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; x -= 1;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; x++;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; ++x;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; x--;"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; --x;"#);
    assert_lint_ok::<PreferConst>(r#"let x; { x = 0; } foo(x);"#);
    assert_lint_ok::<PreferConst>(r#"let x = 0; x = 1;"#);
    assert_lint_ok::<PreferConst>(r#"const x = 0;"#);
    assert_lint_ok::<PreferConst>(
      r#"for (let i = 0, end = 10; i < end; ++i) {}"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"for (let i = 0, end = 10; i < end; i += 2) {}"#,
    );
    assert_lint_ok::<PreferConst>(r#"for (let i in [1,2,3]) { i = 0; }"#);
    assert_lint_ok::<PreferConst>(r#"for (let x of [1,2,3]) { x = 0; }"#);
    assert_lint_ok::<PreferConst>(r#"(function() { var x = 0; })();"#);
    assert_lint_ok::<PreferConst>(r#"(function() { let x; })();"#);
    assert_lint_ok::<PreferConst>(
      r#"(function() { let x; { x = 0; } foo(x); })();"#,
    );
    assert_lint_ok::<PreferConst>(r#"(function() { let x = 0; x = 1; })();"#);
    assert_lint_ok::<PreferConst>(r#"(function() { const x = 0; })();"#);
    assert_lint_ok::<PreferConst>(
      r#"(function() { for (let i = 0, end = 10; i < end; ++i) {} })();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"(function() { for (let i in [1,2,3]) { i = 0; } })();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"(function() { for (let x of [1,2,3]) { x = 0; } })();"#,
    );
    assert_lint_ok::<PreferConst>(r#"(function(x = 0) { })();"#);
    assert_lint_ok::<PreferConst>(r#"let a; while (a = foo());"#);
    assert_lint_ok::<PreferConst>(r#"let a; do {} while (a = foo());"#);
    assert_lint_ok::<PreferConst>(r#"let a; for (; a = foo(); );"#);
    assert_lint_ok::<PreferConst>(r#"let a; for (;; ++a);"#);
    assert_lint_ok::<PreferConst>(r#"let a; for (const {b = ++a} in foo());"#);
    assert_lint_ok::<PreferConst>(r#"let a; for (const {b = ++a} of foo());"#);
    assert_lint_ok::<PreferConst>(
      r#"let a; for (const x of [1,2,3]) { if (a) {} a = foo(); }"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; for (const x of [1,2,3]) { a = a || foo(); bar(a); }"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; for (const x of [1,2,3]) { foo(++a); }"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; function foo() { if (a) {} a = bar(); }"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; function foo() { a = a || bar(); baz(a); }"#,
    );
    assert_lint_ok::<PreferConst>(r#"let a; function foo() { bar(++a); }"#);
    assert_lint_ok::<PreferConst>(
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
    );
    assert_lint_ok::<PreferConst>(r#"let a; function init() { a = foo(); }"#);
    assert_lint_ok::<PreferConst>(r#"let a; if (true) a = 0; foo(a);"#);
    assert_lint_ok::<PreferConst>(
      r#"
        (function (a) {
            let b;
            ({ a, b } = obj);
        })();
        "#,
    );
    assert_lint_ok::<PreferConst>(
      r#"
        (function (a) {
            let b;
            ([ a, b ] = obj);
        })();
        "#,
    );
    assert_lint_ok::<PreferConst>(r#"var a; { var b; ({ a, b } = obj); }"#);
    assert_lint_ok::<PreferConst>(r#"let a; { let b; ({ a, b } = obj); }"#);
    assert_lint_ok::<PreferConst>(r#"var a; { var b; ([ a, b ] = obj); }"#);
    assert_lint_ok::<PreferConst>(r#"let a; { let b; ([ a, b ] = obj); }"#);
    assert_lint_ok::<PreferConst>(r#"let x; { x = 0; foo(x); }"#);
    assert_lint_ok::<PreferConst>(
      r#"(function() { let x; { x = 0; foo(x); } })();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let x; for (const a of [1,2,3]) { x = foo(); bar(x); }"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"(function() { let x; for (const a of [1,2,3]) { x = foo(); bar(x); } })();"#,
    );
    assert_lint_ok::<PreferConst>(r#"let x; for (x of array) { x; }"#);
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [typeNode.returnType, predicate] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [typeNode.returnType, ...predicate] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [typeNode.returnType,, predicate] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [typeNode.returnType=5, predicate] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [[typeNode.returnType=5], predicate] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [[typeNode.returnType, predicate]] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [typeNode.returnType, [predicate]] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [, [typeNode.returnType, predicate]] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [, {foo:typeNode.returnType, predicate}] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let predicate; [, {foo:typeNode.returnType, ...predicate}] = foo();"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; const b = {}; ({ a, c: b.c } = func());"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"const x = [1,2]; let y; [,y] = x; y = 0;"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"const x = [1,2,3]; let y, z; [y,,z] = x; y = 0; z = 0;"#,
    );
    assert_lint_ok::<PreferConst>(r#"try { foo(); } catch (e) {}"#);
    assert_lint_ok::<PreferConst>(
      r#"let a; try { foo(); a = 1; } catch (e) {}"#,
    );
    assert_lint_ok::<PreferConst>(
      r#"let a; try { foo(); } catch (e) { a = 1; }"#,
    );

    // https://github.com/denoland/deno_lint/issues/358
    assert_lint_ok::<PreferConst>(
      r#"let a; const b = (a = foo === bar || baz === qux);"#,
    );
    assert_lint_ok::<PreferConst>(r#"let i = 0; b[i++] = 0;"#);

    // hoisting
    assert_lint_ok::<PreferConst>(
      r#"
      function foo() {
        a += 1;
      }
      let a = 1;
      foo();
      "#,
    );
    assert_lint_ok::<PreferConst>(
      r#"
      let a = 1;
      function foo() {
        function bar() {
          a++;
        }
        let a = 9999;
        bar();
        console.log(a); // 10000
      }
      foo();
      a *= 2;
      console.log(a); // 2
      "#,
    );

    // https://github.com/denoland/deno_lint/issues/358#issuecomment-703587510
    assert_lint_ok::<PreferConst>(r#"let a = 0; for (a in [1, 2, 3]) foo(a);"#);
    assert_lint_ok::<PreferConst>(r#"let a = 0; for (a of [1, 2, 3]) foo(a);"#);
    assert_lint_ok::<PreferConst>(
      r#"let a = 0; for (a = 0; a < 10; a++) foo(a);"#,
    );
  }

  #[test]
  fn prefer_const_invalid() {
    assert_lint_err::<PreferConst>(r#"let x = 1;"#, 4);
    assert_lint_err::<PreferConst>(r#"let x = 1; foo(x);"#, 4);
    assert_lint_err::<PreferConst>(r#"for (let i in [1,2,3]) { foo(i); }"#, 9);
    assert_lint_err::<PreferConst>(r#"for (let x of [1,2,3]) { foo(x); }"#, 9);
    assert_lint_err::<PreferConst>(r#"let [x = -1, y] = [1,2]; y = 0;"#, 5);
    assert_lint_err::<PreferConst>(
      r#"let {a: x = -1, b: y} = {a:1,b:2}; y = 0;"#,
      8,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let x = 1; foo(x); })();"#,
      18,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let i in [1,2,3]) { foo(i); } })();"#,
      23,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let x of [1,2,3]) { foo(x); } })();"#,
      23,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let [x = -1, y] = [1,2]; y = 0; })();"#,
      19,
    );
    assert_lint_err::<PreferConst>(
      r#"let f = (function() { let g = x; })(); f = 1;"#,
      26,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let {a: x = -1, b: y} = {a:1,b:2}; y = 0; })();"#,
      22,
    );
    assert_lint_err::<PreferConst>(
      r#"let x = 0; { let x = 1; foo(x); } x = 0;"#,
      17,
    );
    assert_lint_err::<PreferConst>(
      r#"for (let i = 0; i < 10; ++i) { let x = 1; foo(x); }"#,
      35,
    );
    assert_lint_err_n::<PreferConst>(
      r#"for (let i in [1,2,3]) { let x = 1; foo(x); }"#,
      vec![9, 29],
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
      4,
      11,
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
      4,
      11,
    );
    assert_lint_err::<PreferConst>(r#"let x; x = 0;"#, 4);
    assert_lint_err::<PreferConst>(
      r#"switch (a) { case 0: let x; x = 0; }"#,
      25,
    );
    assert_lint_err::<PreferConst>(r#"(function() { let x; x = 1; })();"#, 18);
    assert_lint_err::<PreferConst>(
      r#"let {a = 0, b} = obj; b = 0; foo(a, b);"#,
      5,
    );
    assert_lint_err::<PreferConst>(
      r#"let {a: {b, c}} = {a: {b: 1, c: 2}}; b = 3;"#,
      12,
    );
    assert_lint_err::<PreferConst>(
      r#"let a, b; ({a = 0, b} = obj); b = 0; foo(a, b);"#,
      4,
    );
    assert_lint_err::<PreferConst>(r#"let [a] = [1]"#, 5);
    assert_lint_err::<PreferConst>(r#"let {a} = obj"#, 5);
    assert_lint_err_n::<PreferConst>(
      r#"let {a = 0, b} = obj, c = a; b = a;"#,
      vec![5, 22],
    );
    assert_lint_err::<PreferConst>(
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      6,
    );
    assert_lint_err::<PreferConst>(
      r#"let x; function foo() { bar(x); } x = 0;"#,
      4,
    );
    assert_lint_err::<PreferConst>(r#"/*eslint use-x:error*/ let x = 1"#, 27);
    assert_lint_err::<PreferConst>(
      r#"/*eslint use-x:error*/ { let x = 1 }"#,
      29,
    );
    assert_lint_err_n::<PreferConst>(r#"let { foo, bar } = baz;"#, vec![11, 6]);
    assert_lint_err::<PreferConst>(r#"const x = [1,2]; let [,y] = x;"#, 23);
    assert_lint_err_n::<PreferConst>(
      r#"const x = [1,2,3]; let [y,,z] = x;"#,
      vec![24, 27],
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}] = foo();"#,
      4,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}, ...bar ] = foo();"#,
      4,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, ...predicate} ] = foo();"#,
      4,
    );
    assert_lint_err_n::<PreferConst>(r#"let x = 'x', y = 'y';"#, vec![4, 13]);
    assert_lint_err::<PreferConst>(r#"let x = 'x', y = 'y'; x = 1"#, 13);
    assert_lint_err_n::<PreferConst>(
      r#"let x = 1, y = 'y'; let z = 1;"#,
      vec![4, 11, 24],
    );
    assert_lint_err_n::<PreferConst>(
      r#"let { a, b, c } = obj; let { x, y, z } = anotherObj; x = 2;"#,
      vec![6, 9, 12, 32, 35],
    );
    assert_lint_err_n::<PreferConst>(
      r#"let x = 'x', y = 'y'; function someFunc() { let a = 1, b = 2; foo(a, b) }"#,
      vec![4, 13, 48, 55],
    );
    assert_lint_err_n::<PreferConst>(
      r#"let someFunc = () => { let a = 1, b = 2; foo(a, b) }"#,
      vec![4, 27, 34],
    );
    assert_lint_err_n::<PreferConst>(r#"let {a, b} = c, d;"#, vec![5, 8]);
    assert_lint_err_n::<PreferConst>(
      r#"let {a, b, c} = {}, e, f;"#,
      vec![5, 8, 11],
    );
    assert_lint_err_on_line_n::<PreferConst>(
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
      vec![(4, 2), (9, 2)],
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
let foo = function(a, b) {
  let c, d, e;
  ({ x: a, y: c } = bar());
  function inner() {
    d = 'd';
  }
  e = 'e';
};
if (true) foo = 'foo';
    "#,
      3,
      12,
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
let e;
try {
  foo();
} catch (e) {
  e = 1;
  e++;
}
e = 2;
    "#,
      2,
      4,
    );
  }
}
