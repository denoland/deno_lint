// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use crate::swc_common::Span;
use crate::swc_common::DUMMY_SP;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::ObjectPatProp;
use crate::swc_ecma_ast::Pat;
use crate::swc_ecma_visit::Node;
use crate::swc_ecma_visit::Visit;
use std::cell::RefCell;
use std::cmp::Eq;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefMut;
use std::cell::Ref;

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
  Program,
  Module,
  Function,
  Block,
  Loop,
  Class,
  Switch,
  With,
  Catch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReferenceKind {
  Read,
  Write,
  ReadWrite,
}

#[derive(Clone, Debug)]
pub struct Reference {
  pub kind: ReferenceKind,
  pub name: String,
}

#[derive(Clone, Debug)]
pub struct ScopeData {
  pub kind: ScopeKind,
  pub parent_scope: Option<Scope>,
  pub span: Span,
  pub child_scopes: Vec<Scope>,
  pub bindings: HashMap<String, BindingKind>,
  pub references: HashMap<String, Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Scope {
  id: usize,
  index: ScopeIndex,
}

impl Scope {
  pub fn new(kind: ScopeKind, span: Span, parent_scope: Option<Self>) -> Self {
    let index = parent_scope
      .as_ref()
      .map(|p| p.index.clone())
      .unwrap_or_else(ScopeIndex::default);
    index.add_scope(ScopeData {
      kind,
      span,
      parent_scope,
      child_scopes: Vec::default(),
      bindings: HashMap::default(),
      references: HashMap::default(),
    })
  }

  pub fn get_kind(&self) -> ScopeKind {
    self.index.borrow()[self.id].kind
  }

  pub fn get_span(&self) -> Span {
    self.index.borrow()[self.id].span
  }

  pub fn get_scope_for_span(&self, span: Span) -> Scope {
    let scopes = self.index.borrow();
    let mut scope_id = self.id;
    'descend: loop {
      let scope_data = &scopes[scope_id];
      for child_scope in &scope_data.child_scopes {
        if child_scope.get_span().contains(span) {
          scope_id = child_scope.id;
          continue 'descend;
        }
      }
      break;
    }
    let scope = Scope {
      id: scope_id,
      index: self.index.clone(),
    };
    assert!(scope.get_span().contains(span));
    scope
  }

  pub fn get_parent_scope(&self) -> Option<Scope> {
    self.index.borrow()[self.id]
      .parent_scope
      .as_ref()
      .map(Scope::clone)
  }

  pub fn add_child_scope(&self, child_scope: &Scope) {
    self.index.borrow_mut()[self.id]
      .child_scopes
      .push(child_scope.clone())
  }

  /// Use a negative value to specify an index that is relative to the end of
  /// the list. E.g. `scope.get_child_scope(-1)` returns the last child scope.
  pub fn get_child_scope(&self, index: isize) -> Option<Scope> {
    let scopes = self.index.borrow();
    let child_scopes = &scopes[self.id].child_scopes;
    let index: usize = if index >= 0 {
      index as usize
    } else {
      child_scopes.len() - (-index as usize)
    };
    child_scopes.get(index).map(Scope::clone)
  }

  pub fn child_scope_count(&self) -> usize {
    self.index.borrow()[self.id].child_scopes.len()
  }

  /// Adds a new binding to this scope. If a binding with the same name has been
  /// added earlier, that binding is removed and returned.
  pub fn add_binding(
    &mut self,
    binding_name: impl Display,
    binding_kind: BindingKind,
  ) -> Option<BindingKind> {
    self.index.borrow_mut()[self.id]
      .bindings
      .insert(binding_name.to_string(), binding_kind)
  }

  pub fn get_binding(
    &self,
    binding_name: impl AsRef<str>,
  ) -> Option<BindingKind> {
    let binding_name = binding_name.as_ref();
    let scopes = self.index.borrow();
    let mut scope_id = self.id;
    loop {
      let scope_data = &scopes[scope_id];
      if let Some(&binding_kind) = scope_data.bindings.get(binding_name) {
        break Some(binding_kind);
      } else if let Some(parent) = &scope_data.parent_scope {
        scope_id = parent.id;
      } else {
        break None;
      }
    }
  }

  pub fn add_reference(&mut self, reference: Reference) {
    let existing = self.index.borrow_mut()[self.id]
      .references
      .insert(reference.name.to_string(), reference);
    assert!(
      existing.is_none(),
      "Trying to add duplicate reference".to_string()
    );
  }

  pub fn get_reference<'a>(&'a self, name: &str) -> Option<Ref<'a, Reference>> {
    let index = self.index.borrow();
      if index[self.id].references.contains_key(name) {
      Some(Ref::map(index, |index| &index[self.id].references[name]))
    } else {
      None
    }
  }

  pub fn get_reference_mut<'a>(&'a self, name: &str) -> Option<RefMut<'a, Reference>> {
    let index = self.index.borrow_mut();
      if index[self.id].references.contains_key(name) {
      Some(RefMut::map(index, |index| index[self.id].references.get_mut(name).unwrap()))
    } else {
      None
    }
  }
}

#[derive(Clone, Debug, Default)]
struct ScopeIndex(Rc<RefCell<Vec<ScopeData>>>);

impl Deref for ScopeIndex {
  type Target = RefCell<Vec<ScopeData>>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl Eq for ScopeIndex {}

impl PartialEq for ScopeIndex {
  fn eq(&self, other: &Self) -> bool {
    Rc::as_ptr(&self.0) == Rc::as_ptr(&other.0)
  }
}

impl Hash for ScopeIndex {
  fn hash<H: Hasher>(&self, state: &mut H) {
    Rc::as_ptr(&self.0).hash(state)
  }
}

impl ScopeIndex {
  fn add_scope(&self, scope_data: ScopeData) -> Scope {
    let mut parent_scope = scope_data.parent_scope.clone();
    let scope = {
      let scopes = &mut self.borrow_mut();
      let id = scopes.len();
      scopes.push(scope_data);
      Scope {
        id,
        index: self.clone(),
      }
    };
    match parent_scope.as_mut() {
      Some(p) => p.add_child_scope(&scope),
      None => assert_eq!(scope.id, 0),
    };
    scope
  }
}

struct PatternVisitor<'sv, F>
where
  F: FnMut(&mut ScopeVisitor, &swc_ecma_ast::Ident, &[Box<swc_ecma_ast::Expr>]),
{
  scope_visitor: &'sv mut ScopeVisitor,
  rhs_nodes: Vec<Box<dyn Node>>,
  #[allow(clippy::vec_box)]
  assignments: Vec<Box<swc_ecma_ast::Expr>>,
  callback: F,
}

impl<
    'sv,
    F: FnMut(&mut ScopeVisitor, &swc_ecma_ast::Ident, &[Box<swc_ecma_ast::Expr>]),
  > PatternVisitor<'sv, F>
{
  fn new(scope_visitor: &'sv mut ScopeVisitor, cb: F) -> Self {
    Self {
      scope_visitor,
      rhs_nodes: vec![],
      assignments: vec![],
      callback: cb,
    }
  }

  fn get_rhs_nodes(self) -> Vec<Box<dyn Node>> {
    self.rhs_nodes
  }
}

impl<
    'sv,
    F: FnMut(&mut ScopeVisitor, &swc_ecma_ast::Ident, &[Box<swc_ecma_ast::Expr>]),
  > Visit for PatternVisitor<'sv, F>
{
  fn visit_pat(&mut self, pat: &swc_ecma_ast::Pat, parent: &dyn Node) {
    // eprintln!("pat {:#?}", pat);
    swc_ecma_visit::visit_pat(self, pat, parent);
  }

  fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident, _parent: &dyn Node) {
    (self.callback)(self.scope_visitor, ident, &self.assignments)
  }

  fn visit_assign_pat(
    &mut self,
    assign_pat: &swc_ecma_ast::AssignPat,
    _parent: &dyn Node,
  ) {
    self.assignments.push(assign_pat.right.clone());
    swc_ecma_visit::visit_pat(self, &*assign_pat.left, assign_pat);
    swc_ecma_visit::visit_expr(
      self.scope_visitor,
      &*assign_pat.right,
      assign_pat,
    );
    self.assignments.pop();
  }

  fn visit_assign_expr(
    &mut self,
    assign_expr: &swc_ecma_ast::AssignExpr,
    _parent: &dyn Node,
  ) {
    self.assignments.push(assign_expr.right.clone());
    swc_ecma_visit::visit_pat_or_expr(self, &assign_expr.left, assign_expr);
    swc_ecma_visit::visit_expr(
      self.scope_visitor,
      &*assign_expr.right,
      assign_expr,
    );
    self.assignments.pop();
  }
}

#[derive(Debug)]
pub struct ScopeVisitor {
  pub scope_stack: Vec<Scope>,
}

impl Default for ScopeVisitor {
  fn default() -> Self {
    let program_scope = Scope::new(ScopeKind::Program, DUMMY_SP, None);
    let scope_stack = vec![program_scope];
    Self { scope_stack }
  }
}

impl ScopeVisitor {
  pub fn enter_scope(&mut self, scope: &Scope) {
    self.scope_stack.push(scope.clone());
  }

  pub fn exit_scope(&mut self, scope: &Scope) {
    assert!(self.scope_stack.len() > 1);
    let exited_scope = self.scope_stack.pop().unwrap();
    assert_eq!(&exited_scope, scope);
  }

  pub fn get_current_scope(&self) -> Scope {
    assert!(!self.scope_stack.is_empty());
    self.scope_stack.last().unwrap().clone()
  }

  pub fn get_root_scope(&self) -> Scope {
    self.scope_stack[0].clone()
  }

  fn create_fn_scope(&mut self, function: &swc_ecma_ast::Function) {
    if let Some(body) = &function.body {
      let fn_scope = Scope::new(
        ScopeKind::Function,
        function.span,
        Some(self.get_current_scope()),
      );
      self.enter_scope(&fn_scope);
      self.visit_function(&function, body);
      self.exit_scope(&fn_scope);
    }
  }

  fn create_getter_or_setter_scope(
    &mut self,
    body: &Option<swc_ecma_ast::BlockStmt>,
  ) {
    if let Some(body) = &body {
      let gs_scope = Scope::new(
        ScopeKind::Function,
        body.span,
        Some(self.get_current_scope()),
      );
      self.enter_scope(&gs_scope);
      for stmt in body.stmts.iter() {
        self.visit_stmt(stmt, body);
      }
      self.exit_scope(&gs_scope);
    }
  }

  fn check_object_lit(&mut self, obj: &swc_ecma_ast::ObjectLit) {
    if obj.props.is_empty() {
      return;
    }
    use crate::swc_ecma_ast::Prop::*;
    for prop in obj.props.iter() {
      if let swc_ecma_ast::PropOrSpread::Prop(prop_expr) = prop {
        match &**prop_expr {
          Method(method_prop) => {
            self.create_fn_scope(&method_prop.function);
          }
          KeyValue(kv_prop) => {
            if let swc_ecma_ast::Expr::Fn(fn_expr) = &*kv_prop.value {
              self.create_fn_scope(&fn_expr.function);
            } else {
              self.check_expr(&kv_prop.value);
            }
          }
          Getter(getter) => {
            self.create_getter_or_setter_scope(&getter.body);
          }
          Setter(setter) => {
            self.create_getter_or_setter_scope(&setter.body);
          }
          _ => {}
        }
      }
    }
  }

  fn check_expr(&mut self, expr: &swc_ecma_ast::Expr) {
    match expr {
      swc_ecma_ast::Expr::Arrow(arrow) => {
        self.visit_block_stmt_or_expr(&arrow.body, arrow);
      }
      swc_ecma_ast::Expr::Object(obj_lit) => {
        self.check_object_lit(&obj_lit);
      }
      swc_ecma_ast::Expr::Array(arr_lit) => {
        self.check_array_lit(&arr_lit);
      }
      _ => {}
    }
  }

  fn check_array_lit(&mut self, arr: &swc_ecma_ast::ArrayLit) {
    if arr.elems.is_empty() {
      return;
    }
    for elem in arr.elems.iter() {
      if let Some(element) = elem {
        if let swc_ecma_ast::Expr::Fn(fn_expr) = &*element.expr {
          self.create_fn_scope(&fn_expr.function);
        } else {
          self.check_expr(&element.expr);
        }
      }
    }
  }

  // /// This function visits `Pat` and calls provided callback for
  // /// each found `Ident`.
  // fn visit_idents_in_pat<'a>(
  //   &mut self,
  //   pat: &'a swc_ecma_ast::Pat,
  //   parent: &'a dyn Node,
  //   callback: Arc<dyn FnMut(&swc_ecma_ast::Ident)>,
  // ) {
  //   let mut pattern_visitor = PatternVisitor::new(callback);
  //   pattern_visitor.visit_pat(pat, parent);
  // }

  fn visit_variable_declaration(
    &mut self,
    decl: &swc_ecma_ast::VarDeclarator,
    parent: &dyn Node,
    kind: BindingKind,
  ) {
    let cb = |sv: &mut ScopeVisitor,
              ident: &swc_ecma_ast::Ident,
              assignments: &[Box<swc_ecma_ast::Expr>]| {
      let mut scope = sv.get_current_scope();
      // TODO(bartlomieju): scope for binding should be passed as an arg
      // to `visit_variable_declaration`
      scope.add_binding(ident.sym.to_string(), kind.clone());

      for assignment in assignments {
        // eprintln!("assignment {:#?}", assignment);
        let ref_ = Reference {
          name: ident.sym.to_string(),
          kind: ReferenceKind::Write,
        };
        scope.add_reference(ref_);
      }

      if decl.init.is_some() {
        let ref_ = Reference {
          name: ident.sym.to_string(),
          kind: ReferenceKind::Write,
        };
        scope.add_reference(ref_);
      }
    };
    let mut pattern_visitor = PatternVisitor::new(self, cb);
    pattern_visitor.visit_pat(&decl.name, parent);
    // let rhs_nodes = pattern_visitor.get_rhs_nodes();
    // for node in rhs_nodes {
    //   swc_ecma_visit::visit(self, node, decl);
    // }
  }

  fn check_pat(&mut self, pat: &Pat, kind: BindingKind) {
    match pat {
      Pat::Ident(ident) => {
        self.get_current_scope().add_binding(&ident.sym, kind);
      }
      Pat::Assign(assign) => {
        self.check_pat(&assign.left, kind);
      }
      Pat::Array(array) => {
        self.check_array_pat(array, kind);
      }
      Pat::Object(object) => {
        self.check_obj_pat(object, kind);
      }
      Pat::Rest(rest) => {
        self.check_pat(&rest.arg, kind);
      }
      _ => {}
    }
  }

  fn check_obj_pat(
    &mut self,
    object: &swc_ecma_ast::ObjectPat,
    kind: BindingKind,
  ) {
    if !object.props.is_empty() {
      for prop in object.props.iter() {
        match prop {
          ObjectPatProp::Assign(assign_prop) => {
            self
              .get_current_scope()
              .add_binding(&assign_prop.key.sym, kind);
          }
          ObjectPatProp::KeyValue(kv_prop) => {
            self.check_pat(&kv_prop.value, kind);
          }
          ObjectPatProp::Rest(rest) => {
            self.check_pat(&rest.arg, kind);
          }
        }
      }
    }
  }

  fn check_array_pat(
    &mut self,
    array: &swc_ecma_ast::ArrayPat,
    kind: BindingKind,
  ) {
    if !array.elems.is_empty() {
      for elem in array.elems.iter() {
        if let Some(element) = elem {
          self.check_pat(element, kind);
        }
      }
    }
  }
}

impl Visit for ScopeVisitor {
  fn visit_module(&mut self, module: &swc_ecma_ast::Module, parent: &dyn Node) {
    let module_scope = Scope::new(
      ScopeKind::Module,
      module.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&module_scope);
    swc_ecma_visit::visit_module(self, module, parent);
    self.exit_scope(&module_scope);
  }

  fn visit_object_lit(
    &mut self,
    obj_lit: &swc_ecma_ast::ObjectLit,
    _parent: &dyn Node,
  ) {
    self.check_object_lit(obj_lit);
  }

  fn visit_array_lit(
    &mut self,
    arr_lit: &swc_ecma_ast::ArrayLit,
    _parent: &dyn Node,
  ) {
    self.check_array_lit(arr_lit);
  }

  fn visit_call_expr(
    &mut self,
    call_expr: &swc_ecma_ast::CallExpr,
    _parent: &dyn Node,
  ) {
    if call_expr.args.is_empty() {
      return;
    }
    for arg in call_expr.args.iter() {
      if let swc_ecma_ast::Expr::Fn(fn_expr) = &*arg.expr {
        self.create_fn_scope(&fn_expr.function);
      } else {
        self.check_expr(&arg.expr)
      }
    }
  }

  fn visit_new_expr(
    &mut self,
    new_expr: &swc_ecma_ast::NewExpr,
    _parent: &dyn Node,
  ) {
    if let Some(args) = &new_expr.args {
      for arg in args.iter() {
        if let swc_ecma_ast::Expr::Fn(fn_expr) = &*arg.expr {
          self.create_fn_scope(&fn_expr.function);
        } else {
          self.check_expr(&arg.expr)
        }
      }
    }
  }

  fn visit_fn_decl(
    &mut self,
    fn_decl: &swc_ecma_ast::FnDecl,
    parent: &dyn Node,
  ) {
    self
      .get_current_scope()
      .add_binding(&fn_decl.ident.sym, BindingKind::Function);
    let fn_scope = Scope::new(
      ScopeKind::Function,
      fn_decl.function.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&fn_scope);
    // swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);
    self.visit_function(&fn_decl.function, fn_decl);
    self.exit_scope(&fn_scope);
  }

  fn visit_class_decl(
    &mut self,
    class_decl: &swc_ecma_ast::ClassDecl,
    parent: &dyn Node,
  ) {
    self
      .get_current_scope()
      .add_binding(&class_decl.ident.sym, BindingKind::Class);
    let class_scope = Scope::new(
      ScopeKind::Class,
      class_decl.class.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&class_scope);
    swc_ecma_visit::visit_class_decl(self, class_decl, parent);
    self.exit_scope(&class_scope);
  }

  fn visit_function(
    &mut self,
    function: &swc_ecma_ast::Function,
    _parent: &dyn Node,
  ) {
    for param in &function.params {
      self.check_pat(&param.pat, BindingKind::Param);
    }

    // Not calling `swc_ecma_visit::visit_function` but instead
    // directly visiting body elements - otherwise additional
    // block scope will be created which is undesirable
    if let Some(body) = &function.body {
      for stmt in &body.stmts {
        swc_ecma_visit::visit_stmt(self, stmt, body);
      }
    }
  }

  fn visit_import_specifier(
    &mut self,
    import_spec: &swc_ecma_ast::ImportSpecifier,
    _parent: &dyn Node,
  ) {
    use crate::swc_ecma_ast::ImportSpecifier::*;
    let local = match import_spec {
      Named(named) => &named.local,
      Default(default) => &default.local,
      Namespace(namespace) => &namespace.local,
    };
    self
      .get_current_scope()
      .add_binding(&local.sym, BindingKind::Import);
  }

  fn visit_expr(&mut self, expr: &swc_ecma_ast::Expr, parent: &dyn Node) {
    dbg!(&expr);
    swc_ecma_visit::visit_expr(self, expr, parent);
  }

  fn visit_assign_expr(
    &mut self,
    assign_expr: &swc_ecma_ast::AssignExpr,
    parent: &dyn Node,
  ) {
    dbg!(&assign_expr);
    swc_ecma_visit::visit_assign_expr(self, assign_expr, parent);
  }

  fn visit_with_stmt(
    &mut self,
    with_stmt: &swc_ecma_ast::WithStmt,
    _parent: &dyn Node,
  ) {
    self.visit_expr(&*with_stmt.obj, with_stmt);

    let with_scope = Scope::new(
      ScopeKind::With,
      with_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&with_scope);
    swc_ecma_visit::visit_stmt(self, &*with_stmt.body, with_stmt);
    self.exit_scope(&with_scope);
  }

  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecma_ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    self.visit_expr(&*switch_stmt.discriminant, switch_stmt);

    let switch_scope = Scope::new(
      ScopeKind::Switch,
      switch_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&switch_scope);
    for case in &switch_stmt.cases {
      swc_ecma_visit::visit_switch_case(self, case, switch_stmt);
    }
    self.exit_scope(&switch_scope);
  }

  fn visit_var_decl(
    &mut self,
    var_decl: &swc_ecma_ast::VarDecl,
    parent: &dyn Node,
  ) {
    use crate::swc_ecma_ast::VarDeclKind;

    let var_kind = match &var_decl.kind {
      VarDeclKind::Var => BindingKind::Var,
      VarDeclKind::Let => BindingKind::Let,
      VarDeclKind::Const => BindingKind::Const,
    };

    for decl in &var_decl.decls {
      self.visit_variable_declaration(decl, parent, var_kind.clone());
      // self.check_pat(&decl.name, var_kind.clone());
      if let Some(boxed_expr) = decl.init.as_ref() {
        swc_ecma_visit::visit_expr(self, &*boxed_expr, decl);
      }
    }
  }

  fn visit_block_stmt(
    &mut self,
    block_stmt: &swc_ecma_ast::BlockStmt,
    parent: &dyn Node,
  ) {
    let block_scope = Scope::new(
      ScopeKind::Block,
      block_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&block_scope);
    swc_ecma_visit::visit_block_stmt(self, block_stmt, parent);
    self.exit_scope(&block_scope);
  }

  fn visit_catch_clause(
    &mut self,
    catch_clause: &swc_ecma_ast::CatchClause,
    _parent: &dyn Node,
  ) {
    let catch_scope = Scope::new(
      ScopeKind::Catch,
      catch_clause.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&catch_scope);

    if let Some(pat) = &catch_clause.param {
      self.check_pat(pat, BindingKind::CatchClause);
    }

    // Not calling `swc_ecma_visit::visit_class` but instead
    // directly visiting body elements - otherwise additional
    // block scope will be created which is undesirable
    for stmt in &catch_clause.body.stmts {
      swc_ecma_visit::visit_stmt(self, stmt, &catch_clause.body);
    }

    self.exit_scope(&catch_scope);
  }

  fn visit_for_stmt(
    &mut self,
    for_stmt: &swc_ecma_ast::ForStmt,
    parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      for_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);
    if let Some(swc_ecma_ast::VarDeclOrExpr::VarDecl(var_decl)) =
      for_stmt.init.as_ref()
    {
      self.visit_var_decl(var_decl, parent);
    }
    if let swc_ecma_ast::Stmt::Block(body_block) = &*for_stmt.body {
      for stmt in &body_block.stmts {
        swc_ecma_visit::visit_stmt(self, &stmt, &for_stmt.body);
      }
    }
    self.exit_scope(&loop_scope);
  }

  fn visit_for_in_stmt(
    &mut self,
    for_in_stmt: &swc_ecma_ast::ForInStmt,
    parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      for_in_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);
    if let swc_ecma_ast::VarDeclOrPat::VarDecl(var_decl) = &for_in_stmt.left {
      self.visit_var_decl(var_decl, parent);
    }
    if let swc_ecma_ast::Stmt::Block(body_block) = &*for_in_stmt.body {
      for stmt in &body_block.stmts {
        swc_ecma_visit::visit_stmt(self, &stmt, &for_in_stmt.body);
      }
    }
    self.exit_scope(&loop_scope);
  }

  fn visit_for_of_stmt(
    &mut self,
    for_of_stmt: &swc_ecma_ast::ForOfStmt,
    parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      for_of_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);
    if let swc_ecma_ast::VarDeclOrPat::VarDecl(var_decl) = &for_of_stmt.left {
      self.visit_var_decl(var_decl, parent);
    }
    if let swc_ecma_ast::Stmt::Block(body_block) = &*for_of_stmt.body {
      for stmt in &body_block.stmts {
        swc_ecma_visit::visit_stmt(self, &stmt, &for_of_stmt.body);
      }
    }
    self.exit_scope(&loop_scope);
  }

  fn visit_while_stmt(
    &mut self,
    while_stmt: &swc_ecma_ast::WhileStmt,
    _parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      while_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);
    if let swc_ecma_ast::Stmt::Block(body_block) = &*while_stmt.body {
      for stmt in &body_block.stmts {
        swc_ecma_visit::visit_stmt(self, &stmt, &while_stmt.body);
      }
    }
    self.exit_scope(&loop_scope);
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &swc_ecma_ast::DoWhileStmt,
    _parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      do_while_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);
    if let swc_ecma_ast::Stmt::Block(body_block) = &*do_while_stmt.body {
      for stmt in &body_block.stmts {
        swc_ecma_visit::visit_stmt(self, &stmt, &do_while_stmt.body);
      }
    }
    self.exit_scope(&loop_scope);
  }

  fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident, _parent: &dyn Node) {
    // eprintln!("visit ident {:#?}", ident);
    let mut scope = self.get_current_scope();
    let ref_ = Reference {
      kind: ReferenceKind::Read,
      name: ident.sym.to_string(),
    };
    scope.add_reference(ref_);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::swc_util;
  use crate::swc_util::AstParser;
  use crate::swc_util::SwcDiagnosticBuffer;

  fn test_scopes(source_code: &str) -> Scope {
    let ast_parser = AstParser::new();
    let syntax = swc_util::get_default_ts_config();
    ast_parser
      .parse_module(
        "file_name.ts",
        syntax,
        source_code,
        |parse_result, _comments| -> Result<Scope, SwcDiagnosticBuffer> {
          let module = parse_result?;
          let mut scope_visitor = ScopeVisitor::default();
          let root_scope = scope_visitor.get_root_scope();
          scope_visitor.visit_module(&module, &module);
          Ok(root_scope)
        },
      )
      .unwrap()
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
    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 4);

    let fn_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(fn_scope.get_kind(), ScopeKind::Function);
    assert_eq!(fn_scope.child_scope_count(), 1);

    let block_scope = fn_scope.get_child_scope(0).unwrap();
    assert_eq!(block_scope.get_kind(), ScopeKind::Block);
    assert_eq!(block_scope.child_scope_count(), 0);

    let class_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(class_scope.get_kind(), ScopeKind::Class);
    assert_eq!(class_scope.child_scope_count(), 0);

    let catch_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(catch_scope.get_kind(), ScopeKind::Catch);
    assert_eq!(catch_scope.child_scope_count(), 0);
    let catch_clause_e = catch_scope.get_binding("e").unwrap();
    assert_eq!(catch_clause_e, BindingKind::CatchClause);
  }

  #[test]
  fn switch_scope() {
    let source_code = r#"
switch (foo) {
  case "foo":
    let a = "a";
    a = "b";
    a;
    break;
  case "bar":
    break;
  default:
    const defaultVal = "default";
    return defaultVal;
}
"#;
    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 1);

    let switch_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(switch_scope.get_kind(), ScopeKind::Switch);
    assert_eq!(switch_scope.child_scope_count(), 0);
    assert!(switch_scope.get_binding("a").is_some());
    assert!(switch_scope.get_binding("defaultVal").is_some());
  }
  #[test]
  fn loop_scopes() {
    let source_code = r#"
    for (let i = 0; i < 10; i++){}
    for (let i in [1,2,3]){}
    for (let i of [1,2,3]){}
    while (i > 1) {}
    do {} while (i > 1)
"#;
    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 5);

    let for_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(for_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(for_scope.child_scope_count(), 0);
    assert!(for_scope.get_binding("i").is_some());

    let for_in_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(for_in_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(for_in_scope.child_scope_count(), 0);
    assert!(for_in_scope.get_binding("i").is_some());

    let for_of_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(for_of_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(for_of_scope.child_scope_count(), 0);
    assert!(for_of_scope.get_binding("i").is_some());

    let while_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(while_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(while_scope.child_scope_count(), 0);

    let do_while_scope = module_scope.get_child_scope(-1).unwrap();
    assert_eq!(do_while_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(do_while_scope.child_scope_count(), 0);
  }

  #[test]
  fn call_new_expressions() {
    let source_code = r#"
    Deno.test("first test", function(){});
    new Deno(function(){});
    "#;

    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 2);

    let call_fn_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(call_fn_scope.get_kind(), ScopeKind::Function);

    let new_fn_scope = module_scope.get_child_scope(-1).unwrap();
    assert_eq!(new_fn_scope.get_kind(), ScopeKind::Function);
  }

  #[test]
  fn object_literal() {
    let source_code = r#"
    let obj = {
      method(){
        const e;
      },
      nested : {
        nestedMethod(){
          const f;
        }
      },
      getterAndSetter : {
        get getter(){
          const g;
          return g;
        },
        set setter(s){
          const h;
        }
      }
    }
    "#;
    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 4);

    let obj_method_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(obj_method_scope.get_kind(), ScopeKind::Function);
    assert!(obj_method_scope.get_binding("e").is_some());

    let obj_nested_method_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(obj_nested_method_scope.get_kind(), ScopeKind::Function);
    assert!(obj_nested_method_scope.get_binding("f").is_some());

    let obj_getter_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(obj_getter_scope.get_kind(), ScopeKind::Function);
    assert!(obj_getter_scope.get_binding("g").is_some());
    assert!(obj_getter_scope.get_binding("h").is_none());

    let obj_setter_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(obj_setter_scope.get_kind(), ScopeKind::Function);
    assert!(obj_setter_scope.get_binding("h").is_some());
    assert!(obj_setter_scope.get_binding("g").is_none());
  }

  #[test]
  fn array_literal() {
    let source_code = r#"
    let array = [
      function x(){ const a; },
      ()=>{const b;},
      [
        function nested() { const c;}
      ],
      {
        innerMethod(){
          const d;
        }
      }
    ]
    "#;

    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 4);

    let array_fn_scope = module_scope.get_child_scope(0).unwrap();
    assert_eq!(array_fn_scope.get_kind(), ScopeKind::Function);
    assert!(array_fn_scope.get_binding("a").is_some());
    assert!(array_fn_scope.get_binding("b").is_none());

    let array_arrow_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(array_arrow_scope.get_kind(), ScopeKind::Block);
    assert!(array_arrow_scope.get_binding("b").is_some());
    assert!(array_arrow_scope.get_binding("c").is_none());

    let array_nested_fn_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(array_nested_fn_scope.get_kind(), ScopeKind::Function);
    assert!(array_nested_fn_scope.get_binding("c").is_some());

    let array_object_method_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(array_object_method_scope.get_kind(), ScopeKind::Function);
    assert!(array_object_method_scope.get_binding("d").is_some());
  }

  #[test]
  fn import_binding() {
    let source_code = r#"
    import defaultExport1 from "module-name";
    import * as namespaced1 from "module-name";
    import { export1 } from "module-name";
    import { export2 as alias1 } from "module-name";
    import { export3 , export4 } from "module-name";
    import { export5 , export6 as alias2} from "module-name";
    import defaultExport2, { export7 } from "module-name";
    import defaultExport3, * as namespaced2 from "module-name";
    import "module-name";
    var promise = import("module-name");
    "#;

    let root_scope = test_scopes(source_code);
    assert_eq!(root_scope.get_kind(), ScopeKind::Program);
    assert_eq!(root_scope.child_scope_count(), 1);

    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert_eq!(module_scope.get_kind(), ScopeKind::Module);
    assert_eq!(module_scope.child_scope_count(), 0);
    let default_export = module_scope.get_binding("defaultExport1").unwrap();
    assert_eq!(default_export, BindingKind::Import);
    let namespaced1 = module_scope.get_binding("namespaced1").unwrap();
    assert_eq!(namespaced1, BindingKind::Import);
    let export1 = module_scope.get_binding("export1").unwrap();
    assert_eq!(export1, BindingKind::Import);
    assert!(module_scope.get_binding("export2").is_none());
    assert!(module_scope.get_binding("alias1").is_some());
    assert!(module_scope.get_binding("export3").is_some());
    assert!(module_scope.get_binding("export4").is_some());
    assert!(module_scope.get_binding("export5").is_some());
    assert!(module_scope.get_binding("export6").is_none());
    assert!(module_scope.get_binding("alias2").is_some());
    assert!(module_scope.get_binding("defaultExport2").is_some());
    assert!(module_scope.get_binding("export7").is_some());
    assert!(module_scope.get_binding("defaultExport3").is_some());
    assert!(module_scope.get_binding("namespaced2").is_some());
  }

  #[test]
  fn destructuring_assignment() {
    let source_code = r#"
const {a} = {a : "a"};
const {a: {b}} = {a : {b: "b"}};
const {a: {b: {c}}} = {a : {b: {c : "c"}}};
const [d] = ["d"];
const [e, [f,[g]]] = ["e",["f",["g"]]];
const {a: [h]} = {a: ["h"]};
const [i, {j}] = ["i",{j: "j"}];
const {a: {b : [k,{l}]}} = {a: {b : ["k",{l : "l"}]}};
const {m = "M"} = {};
const [n = "N"] = [];
const {...o} = {o : "O"};
const [...p] = ["p"];
function getPerson({username="disizali",info: [name, family]}) {
  try {
    throw 'TryAgain';
  } catch(e) {}
}
try{} catch({message}){};
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    assert!(module_scope.get_binding("a").is_some());
    assert!(module_scope.get_binding("b").is_some());
    assert!(module_scope.get_binding("c").is_some());
    assert!(module_scope.get_binding("d").is_some());
    assert!(module_scope.get_binding("e").is_some());
    assert!(module_scope.get_binding("f").is_some());
    assert!(module_scope.get_binding("g").is_some());
    assert!(module_scope.get_binding("h").is_some());
    assert!(module_scope.get_binding("i").is_some());
    assert!(module_scope.get_binding("j").is_some());
    assert!(module_scope.get_binding("k").is_some());
    assert!(module_scope.get_binding("l").is_some());
    assert!(module_scope.get_binding("m").is_some());
    assert!(module_scope.get_binding("n").is_some());
    assert!(module_scope.get_binding("o").is_some());
    assert!(module_scope.get_binding("p").is_some());

    let function_scope = module_scope.get_child_scope(0).unwrap();
    assert!(function_scope.get_binding("username").is_some());
    assert!(function_scope.get_binding("name").is_some());
    assert!(function_scope.get_binding("family").is_some());

    let function_catch_scope = function_scope.get_child_scope(-1).unwrap();
    assert!(function_catch_scope.get_binding("e").is_some());

    let catch_scope = module_scope.get_child_scope(-1).unwrap();
    assert!(catch_scope.get_binding("message").is_some());
  }

  #[test]
  fn references() {
    let source_code = r#"
let a = 0;
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    // assert_eq!(module_scope.bindings.len(), 1);
    assert!(module_scope.get_binding("a").is_some());
    // assert_eq!(module_scope.references.len(), 1);
    let a_ref = module_scope.get_reference("a").unwrap();
    assert_eq!(a_ref.kind, ReferenceKind::Write);
  }

  #[test]
  fn references2() {
    let source_code = r#"
let a = 0;
function b() {
  let c = a;
}
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    // assert_eq!(module_scope.bindings.len(), 2);
    assert!(module_scope.get_binding("a").is_some());
    assert!(module_scope.get_binding("b").is_some());

    // assert_eq!(module_scope.references.len(), 1); // a
    let a_ref = module_scope.get_reference("a").unwrap();
    assert_eq!(a_ref.kind, ReferenceKind::Write);

    let fn_scope = module_scope.get_child_scope(0).unwrap();
    // eprintln!("function scope refs {:#?}", fn_scope);
    // assert_eq!(fn_scope.bindings.len(), 1); // b
    assert!(fn_scope.get_binding("c").is_some());

    // assert_eq!(fn_scope.references.len(), 2); // c, a
    let a_ref = fn_scope.get_reference("c").unwrap();
    assert_eq!(a_ref.kind, ReferenceKind::Write);
    let a_ref = fn_scope.get_reference("a").unwrap();
    assert_eq!(a_ref.kind, ReferenceKind::Read);
  }
}
