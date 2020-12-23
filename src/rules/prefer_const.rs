// Copyright 2020 the Deno authors. All rights reserved. MIT license.
// TODO(magurotuna): delete this directive
#![allow(unused)]
use super::Context;
use super::LintRule;
use derive_more::Display;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::rc::Rc;
use swc_atoms::JsWord;
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrowExpr, AssignExpr, BlockStmt, BlockStmtOrExpr, CatchClause, Class,
  Constructor, DoWhileStmt, Expr, ExprStmt, ForInStmt, ForOfStmt, ForStmt,
  Function, Ident, IfStmt, ObjectPatProp, ParamOrTsParamProp, Pat, PatOrExpr,
  Program, Stmt, TsParamPropParam, UpdateExpr, VarDecl, VarDeclKind,
  VarDeclOrExpr, VarDeclOrPat, WhileStmt, WithStmt,
};
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::{Node, Visit, VisitWith};

pub struct PreferConst;

const CODE: &str = "prefer-const";

#[derive(Display)]
enum PreferConstMessage {
  #[display(fmt = "'{}' is never reassigned", _0)]
  NeverReassigned(String),
}

#[derive(Display)]
enum PreferConstHint {
  #[display(fmt = "Use 'const' instead")]
  UseConst,
}

impl LintRule for PreferConst {
  fn new() -> Box<Self> {
    Box::new(PreferConst)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut collector = VariableCollector::new();
    collector.visit_program(program, program);

    let mut visitor =
      PreferConstVisitor::new(context, mem::take(&mut collector.scopes));
    visitor.visit_program(program, program);
  }
}

type Scope = Rc<RefCell<RawScope>>;

#[derive(Debug)]
struct RawScope {
  parent: Option<Scope>,
  variables: BTreeMap<JsWord, Span>,
}

impl RawScope {
  fn new(parent: Option<Scope>) -> Self {
    Self {
      parent,
      variables: BTreeMap::new(),
    }
  }
}

/// Looks for the span of the given variable by traversing from the given scope to the parents.
/// Returns `None` if no matching span is found. Most likely it means the variable is not declared
/// with `let`.
fn get_span_by_ident(scope: Scope, ident: &Ident) -> Option<Span> {
  let mut cur_scope = Some(scope);
  while let Some(cur) = cur_scope {
    if let Some(span) = cur.borrow().variables.get(&ident.sym) {
      return Some(*span);
    }
    cur_scope = cur.borrow().parent.as_ref().map(Rc::clone);
  }
  None
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum VarStatus {
  Declared,
  Initialized,
  Mutated,
}

impl VarStatus {
  fn next(self) -> Self {
    use VarStatus::*;
    match self {
      Declared => Initialized,
      Initialized => Mutated,
      Mutated => Mutated,
    }
  }
}

#[derive(Default, Debug)]
struct DisjointSet {
  /// Key: span of ident, Value: span of parent node
  /// This map is supposed to contain all spans of variable declarations.
  parents: HashMap<Span, Span>,

  /// Key: span of ident (representative of the group)
  /// Value: pair of the following values:
  ///        - status of variables in this tree
  ///        - the maximum height of this tree, which is used for optimization
  roots: HashMap<Span, (VarStatus, usize)>,
}

impl DisjointSet {
  fn new() -> Self {
    Self::default()
  }

  fn add_root(&mut self, span: Span, status: VarStatus) {
    if self.parents.contains_key(&span) {
      return;
    }
    self.parents.insert(span, span);
    self.roots.insert(span, (status, 1));
  }

  fn get_root(&mut self, span: Span) -> Option<Span> {
    match self.parents.get(&span) {
      None => None,
      Some(&par_span) if span == par_span => Some(span),
      Some(&par_span) => {
        let root = self.get_root(par_span);
        match (root, self.parents.get_mut(&span)) {
          (Some(root_span), Some(par)) => {
            *par = root_span;
          }
          _ => {}
        }
        root
      }
    }
  }

  fn unite(&mut self, span1: Span, span2: Span) -> Option<()> {
    let rs1 = self.get_root(span1)?;
    let rs2 = self.get_root(span2)?;
    if rs1 == rs2 {
      return None;
    }

    let &(status1, rank1) = self.roots.get(&rs1)?;
    let &(status2, rank2) = self.roots.get(&rs2)?;

    let next_status = std::cmp::max(status1, status2);

    if rank1 <= rank2 {
      let p = self.parents.get_mut(&rs1)?;
      *p = rs2;
      let r = self.roots.get_mut(&rs2)?;
      *r = (next_status, std::cmp::max(rank1 + 1, rank2));
      self.roots.remove(&rs1);
    } else {
      let p = self.parents.get_mut(&rs2)?;
      *p = rs1;
      let r = self.roots.get_mut(&rs1)?;
      *r = (next_status, rank1);
      self.roots.remove(&rs2);
    }

    Some(())
  }

  fn dump(mut self) -> Vec<Span> {
    self
      .parents
      .clone()
      .keys()
      .filter_map(|&cur| {
        let root = self.get_root(cur)?;
        if matches!(self.roots.get(&root), Some((VarStatus::Initialized, _))) {
          Some(cur)
        } else {
          None
        }
      })
      .collect()
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
  var_groups: DisjointSet,
}

impl VariableCollector {
  fn new() -> Self {
    Self {
      scopes: BTreeMap::new(),
      cur_scope: ScopeRange::Global,
      var_groups: DisjointSet::new(),
    }
  }

  fn insert_var(&mut self, ident: &Ident, status: VarStatus) {
    self.var_groups.add_root(ident.span, status);
    let mut scope = self.scopes.get(&self.cur_scope).unwrap().borrow_mut();
    scope.variables.insert(ident.sym.clone(), ident.span);
  }

  fn extract_decl_idents(&mut self, pat: &Pat, has_init: bool) {
    let status = if has_init {
      VarStatus::Initialized
    } else {
      VarStatus::Declared
    };

    match pat {
      Pat::Ident(ident) => self.insert_var(ident, status),
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
                self.insert_var(&assign.key, VarStatus::Initialized);
              } else {
                self.insert_var(&assign.key, status);
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

  fn with_child_scope<F, S>(&mut self, node: S, op: F)
  where
    S: Spanned,
    F: FnOnce(&mut VariableCollector),
  {
    let parent_scope_range = self.cur_scope;
    let parent_scope = self.scopes.get(&parent_scope_range).map(Rc::clone);
    let child_scope = RawScope::new(parent_scope);
    self.scopes.insert(
      ScopeRange::Block(node.span()),
      Rc::new(RefCell::new(child_scope)),
    );
    self.cur_scope = ScopeRange::Block(node.span());
    op(self);
    self.cur_scope = parent_scope_range;
  }
}

impl Visit for VariableCollector {
  noop_visit_type!();

  fn visit_program(&mut self, program: &Program, _: &dyn Node) {
    let scope = RawScope::new(None);
    self
      .scopes
      .insert(ScopeRange::Global, Rc::new(RefCell::new(scope)));
    program.visit_children_with(self);
  }

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    self.with_child_scope(function, |a| {
      for param in &function.params {
        param.visit_children_with(a);
        let idents: Vec<Ident> = find_ids(&param.pat);
        for ident in idents {
          a.insert_var(&ident, VarStatus::Mutated);
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
          a.insert_var(&ident, VarStatus::Mutated);
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
              a.extract_decl_idents(&decl.name, decl.init.is_some());
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
            a.extract_decl_idents(&decl.name, true);
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
            a.extract_decl_idents(&decl.name, true);
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
          a.insert_var(&ident, VarStatus::Mutated);
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
                a.insert_var(ident, VarStatus::Mutated);
              }
              TsParamPropParam::Assign(assign_pat) => {
                assign_pat.visit_children_with(a);
                let idents: Vec<Ident> = find_ids(&assign_pat.left);
                for ident in idents {
                  a.insert_var(&ident, VarStatus::Mutated);
                }
              }
            }
          }
          ParamOrTsParamProp::Param(param) => {
            param.visit_children_with(a);
            let idents: Vec<Ident> = find_ids(&param.pat);
            for ident in idents {
              a.insert_var(&ident, VarStatus::Mutated);
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
        self.extract_decl_idents(&decl.name, decl.init.is_some());
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
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      PreferConstMessage::NeverReassigned(sym.to_string()),
      PreferConstHint::UseConst,
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
    let _scope = self.scopes.get(&self.cur_scope).unwrap();
    return;
    // TODO(magurotuna)
    //update_variable_status(Rc::clone(scope), ident, force_reassigned);
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
      // If the pat contains either of the following:
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
    let mut cur_scope = self.scopes.get(&self.cur_scope).map(Rc::clone);
    let mut is_first_loop = true;
    while let Some(cur) = cur_scope {
      let lock = cur.borrow();
      if let Some(var) = lock.variables.get(&ident.sym) {
        if is_first_loop {
          //return var.is_param;
          return true; // TODO(magurotuna) wip
        } else {
          return true;
        }
      }
      is_first_loop = false;
      cur_scope = lock.parent.as_ref().map(Rc::clone);
    }
    // If the declaration isn't found, most likely it means the ident is declared with `var`
    false
  }

  fn exit_program(&mut self) {
    // TODO(magurotuna) wip
    return;
    //let mut for_init_vars = BTreeMap::new();
    //let mut destructuring_vars = BTreeMap::new();
    //let scopes = self.scopes.clone();
    //for scope in scopes.values() {
    //for (sym, status) in scope.borrow().variables.iter() {
    //return;
    //match status.handling {
    //Handling::Normal => {
    //if status.should_report() {
    //self.report(sym, status.span);
    //}
    //}
    //Handling::ForInit(for_span) => {
    //for_init_vars
    //.entry(for_span)
    //.or_insert_with(Vec::new)
    //.push((sym.clone(), *status));
    //}
    //Handling::Destructuring(dest_span) => {
    //destructuring_vars
    //.entry(dest_span)
    //.or_insert_with(Vec::new)
    //.push((sym.clone(), *status));
    //}
    //}
    //}
    //}

    // For special cases, we should report diagnostics only if *all* variables there need to be
    // reported.
    //let mut check_special_case =
    //|vars: BTreeMap<Span, Vec<(JsWord, Variable)>>| {
    //for (sym, var) in vars
    //.iter()
    //.filter_map(|(_, vars)| {
    //if vars.iter().all(|(_, status)| status.should_report()) {
    //Some(vars)
    //} else {
    //None
    //}
    //})
    //.flatten()
    //{
    //self.report(sym, var.span);
    //}
    //};

    //check_special_case(for_init_vars);
    //check_special_case(destructuring_vars);
  }
}

impl<'c> Visit for PreferConstVisitor<'c> {
  noop_visit_type!();

  fn visit_program(&mut self, program: &Program, _: &dyn Node) {
    program.visit_children_with(self);
    self.exit_program();
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
    let program = test_util::parse(src);
    let mut v = VariableCollector::new();
    v.visit_program(&program, &program);
    v
  }

  fn variables(scope: &Scope) -> Vec<String> {
    scope
      .borrow()
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

  fn belongs_to_same_group(
    ds: &mut DisjointSet,
    span1: Span,
    span2: Span,
  ) -> bool {
    let r1 = ds.get_root(span1);
    let r2 = ds.get_root(span2);
    matches!((r1, r2), (Some(x), Some(y)) if x == y)
  }

  #[test]
  fn var_groups_1() {
    let src = r#"
let { foo, bar } = obj;
let baz = 42;
    "#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 2);
    assert_eq!(v.var_groups.dump().len(), 3);
  }
}

#[cfg(test)]
mod prefer_const_tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/prefer-const.js
  // MIT Licensed.

  #[test]
  fn prefer_const_valid() {
    assert_lint_ok! {
      PreferConst,
      r#"var x = 0;"#,
      r#"let x;"#,
      r#"let x = 0; x += 1;"#,
      r#"let x = 0; x -= 1;"#,
      r#"let x = 0; x++;"#,
      r#"let x = 0; ++x;"#,
      r#"let x = 0; x--;"#,
      r#"let x = 0; --x;"#,
      r#"let x; { x = 0; } foo(x);"#,
      r#"let x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"for (let i = 0, end = 10; i < end; ++i) {}"#,
      r#"for (let i = 0, end = 10; i < end; i += 2) {}"#,
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
      r#"let a; function init() { a = foo(); }"#,
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
      r#"try { foo(); } catch (e) {}"#,
      r#"let a; try { foo(); a = 1; } catch (e) {}"#,
      r#"let a; try { foo(); } catch (e) { a = 1; }"#,

      // https://github.com/denoland/deno_lint/issues/358
      r#"let a; const b = (a = foo === bar || baz === qux);"#,
      r#"let i = 0; b[i++] = 0;"#,

      // hoisting
      r#"
      function foo() {
        a += 1;
      }
      let a = 1;
      foo();
      "#,
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

      // https://github.com/denoland/deno_lint/issues/358#issuecomment-703587510
      r#"let a = 0; for (a in [1, 2, 3]) foo(a);"#,
      r#"let a = 0; for (a of [1, 2, 3]) foo(a);"#,
      r#"let a = 0; for (a = 0; a < 10; a++) foo(a);"#,

      // https://github.com/denoland/deno_lint/issues/522
      // imitates `{ "destructuring": "all" }` in ESLint
      r#"let {a, b} = obj; b = 0;"#,
      r#"let a, b; ({a, b} = obj); b++;"#,
      r#"let a, b, c; ({a, b} = obj1); ({b, c} = obj2) b++;"#,
      r#"let {a = 0, b} = obj; b = 0; foo(a, b);"#,
      r#"let {a: {b, c}} = {a: {b: 1, c: 2}}; b = 3;"#,
      r#"let a, b; ({a = 0, b} = obj); b = 0; foo(a, b);"#,
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      r#"(function() { let {a: x = -1, b: y} = {a:1,b:2}; y = 0; })();"#,
      r#"(function() { let [x = -1, y] = [1,2]; y = 0; })();"#,
      r#"let {a: x = -1, b: y} = {a:1,b:2}; y = 0;"#,
      r#"let [x = -1, y] = [1,2]; y = 0;"#,
    };
  }

  #[test]
  fn prefer_const_invalid() {
    assert_lint_err! {
      PreferConst,
      r#"let x = 1;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 1; foo(x);"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i in [1,2,3]) { foo(i); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let x of [1,2,3]) { foo(x); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { let x = 1; foo(x); })();"#: [
        {
          col: 18,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { for (let i in [1,2,3]) { foo(i); } })();"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { for (let x of [1,2,3]) { foo(x); } })();"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let f = (function() { let g = x; })(); f = 1;"#: [
        {
          col: 26,
          message: variant!(PreferConstMessage, NeverReassigned, "g"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 0; { let x = 1; foo(x); } x = 0;"#: [
        {
          col: 17,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i = 0; i < 10; ++i) { let x = 1; foo(x); }"#: [
        {
          col: 35,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i in [1,2,3]) { let x = 1; foo(x); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 29,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       a = 1;
   }
};
    "#: [
        {
          line: 4,
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       ({a} = 1);
   }
};
    "#: [
        {
          line: 4,
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x; x = 0;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"switch (a) { case 0: let x; x = 0; }"#: [
        {
          col: 25,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { let x; x = 1; })();"#: [
        {
          col: 18,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let [a] = [1]"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a} = obj"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a = 0, b} = obj, c = a; b = a;"#: [
        {
          col: 22,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x; function foo() { bar(x); } x = 0;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"/*eslint use-x:error*/ let x = 1"#: [
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"/*eslint use-x:error*/ { let x = 1 }"#: [
        {
          col: 29,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let { foo, bar } = baz;"#: [
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 6,
          message: variant!(PreferConstMessage, NeverReassigned, "foo"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"const x = [1,2]; let [,y] = x;"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"const x = [1,2,3]; let [y,,z] = x;"#: [
        {
          col: 24,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "z"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, predicate}] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, predicate}, ...bar ] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, ...predicate} ] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 'x', y = 'y';"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 'x', y = 'y'; x = 1"#: [
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 1, y = 'y'; let z = 1;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 24,
          message: variant!(PreferConstMessage, NeverReassigned, "z"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let { a, b, c } = obj; let { x, y, z } = anotherObj; x = 2;"#: [
        {
          col: 6,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 12,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        },
      ],
      r#"let x = 'x', y = 'y'; function someFunc() { let a = 1, b = 2; foo(a, b) }"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 48,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 55,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let someFunc = () => { let a = 1, b = 2; foo(a, b) }"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "someFunc"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 34,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a, b} = c, d;"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 8,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a, b, c} = {}, e, f;"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 8,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        }
      ],
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
    "#: [
        {
          line: 4,
          col: 2,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        },
        {
          line: 9,
          col: 2,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        }
      ],
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
    "#: [
        {
          line: 3,
          col: 12,
          message: variant!(PreferConstMessage, NeverReassigned, "e"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
let e;
try {
  foo();
} catch (e) {
  e = 1;
  e++;
}
e = 2;
    "#: [
        {
          line: 2,
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "e"),
          hint: PreferConstHint::UseConst,
        }
      ]
    };
  }
}
