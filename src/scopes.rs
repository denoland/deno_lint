// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use crate::swc_common::Span;
use crate::swc_common::DUMMY_SP;
use crate::swc_ecma_ast;
use crate::swc_ecma_visit::Node;
use crate::swc_ecma_visit::Visit;
use std::cell::Ref;
use std::cell::RefCell;
use std::cmp::Eq;
use std::cmp::PartialEq;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::rc::Rc;

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

#[derive(Clone, Debug)]
pub struct Binding {
  pub kind: BindingKind,
  pub name: String,
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
  pub bindings: Vec<Binding>,
  pub references: Vec<Reference>,
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
      bindings: Vec::default(),
      references: Vec::default(),
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

  pub fn add_binding(&mut self, binding: Binding) {
    self.index.borrow_mut()[self.id].bindings.push(binding);
  }

  pub fn get_bindings<'a>(&'a self) -> Ref<'a, Vec<Binding>> {
    let index = self.index.borrow();
    Ref::map(index, |index| &index[self.id].bindings)
  }

  pub fn add_reference(&mut self, reference: Reference) {
    let data = &mut self.index.borrow_mut()[self.id];
    data.references.push(reference);
  }

  pub fn get_references<'a>(&'a self) -> Ref<'a, Vec<Reference>> {
    let index = self.index.borrow();
    Ref::map(index, |index| &index[self.id].references)
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
  #[allow(unused)]
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

  #[allow(unused)]
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
    eprintln!("pat {:#?}", pat);
    swc_ecma_visit::visit_pat(self, pat, parent);
  }

  fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident, _parent: &dyn Node) {
    // eprintln!("visit ident! {:#?}", ident);
    (self.callback)(self.scope_visitor, ident, &self.assignments)
  }

  fn visit_assign_pat(
    &mut self,
    assign_pat: &swc_ecma_ast::AssignPat,
    _parent: &dyn Node,
  ) {
    eprintln!("visit assign pat {:#?}", assign_pat);
    self.assignments.push(assign_pat.right.clone());
    swc_ecma_visit::visit_pat(self, &*assign_pat.left, assign_pat);
    self
      .scope_visitor
      .visit_expr(&*assign_pat.right, assign_pat);
    self.assignments.pop();
  }

  fn visit_assign_expr(
    &mut self,
    assign_expr: &swc_ecma_ast::AssignExpr,
    _parent: &dyn Node,
  ) {
    self.assignments.push(assign_expr.right.clone());
    swc_ecma_visit::visit_pat_or_expr(self, &assign_expr.left, assign_expr);
    self
      .scope_visitor
      .visit_expr(&*assign_expr.right, assign_expr);
    self.assignments.pop();
  }

  fn visit_assign_pat_prop(
    &mut self,
    assign_pat_prop: &swc_ecma_ast::AssignPatProp,
    _parent: &dyn Node,
  ) {
    let mut has_assignment = false;
    if let Some(boxed_expr) = assign_pat_prop.value.as_ref() {
      self.assignments.push(boxed_expr.clone());
      has_assignment = true;
    }
    self.visit_ident(&assign_pat_prop.key, assign_pat_prop);
    if has_assignment {
      self.assignments.pop();
    }
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
      scope.add_binding(Binding {
        name: ident.sym.to_string(),
        kind,
      });

      for _assignment in assignments {
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
    _parent: &dyn Node,
  ) {
    self.get_current_scope().add_binding(Binding {
      name: fn_decl.ident.sym.to_string(),
      kind: BindingKind::Function,
    });
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
    self.get_current_scope().add_binding(Binding {
      name: class_decl.ident.sym.to_string(),
      kind: BindingKind::Class,
    });
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
      let cb = |sv: &mut ScopeVisitor,
                ident: &swc_ecma_ast::Ident,
                assignments: &[Box<swc_ecma_ast::Expr>]| {
        let mut scope = sv.get_current_scope();
        scope.add_binding(Binding {
          name: ident.sym.to_string(),
          kind: BindingKind::Param,
        });

        for _assignment in assignments {
          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
        }
      };
      let mut pattern_visitor = PatternVisitor::new(self, cb);
      pattern_visitor.visit_pat(&param.pat, param);
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
    self.get_current_scope().add_binding(Binding {
      name: local.sym.to_string(),
      kind: BindingKind::Import,
    });
  }

  fn visit_expr(&mut self, expr: &swc_ecma_ast::Expr, parent: &dyn Node) {
    // dbg!(&expr);
    swc_ecma_visit::visit_expr(self, expr, parent);
  }

  fn visit_assign_expr(
    &mut self,
    assign_expr: &swc_ecma_ast::AssignExpr,
    _parent: &dyn Node,
  ) {
    use swc_ecma_ast::AssignOp;
    use swc_ecma_ast::PatOrExpr;
    let cb = |sv: &mut ScopeVisitor,
              ident: &swc_ecma_ast::Ident,
              assignments: &[Box<swc_ecma_ast::Expr>]| {
      let mut scope = sv.get_current_scope();
      let ref_ = Reference {
        name: ident.sym.to_string(),
        kind: if assign_expr.op == AssignOp::Assign {
          ReferenceKind::Write
        } else {
          ReferenceKind::ReadWrite
        },
      };
      scope.add_reference(ref_);

      if assign_expr.op == AssignOp::Assign {
        for _assignment in assignments {
          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
        }
      }
    };
    let mut pattern_visitor = PatternVisitor::new(self, cb);
    match &assign_expr.left {
      PatOrExpr::Pat(boxed_pat) => {
        pattern_visitor.visit_pat(&*boxed_pat, assign_expr);
      }
      PatOrExpr::Expr(boxed_expr) => {
        pattern_visitor.visit_expr(&*boxed_expr, assign_expr);
      }
    };

    self.visit_expr(&*assign_expr.right, assign_expr);
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
      self.visit_variable_declaration(decl, parent, var_kind);
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
      let cb = |sv: &mut ScopeVisitor,
                ident: &swc_ecma_ast::Ident,
                assignments: &[Box<swc_ecma_ast::Expr>]| {
        let mut scope = sv.get_current_scope();
        scope.add_binding(Binding {
          name: ident.sym.to_string(),
          kind: BindingKind::CatchClause,
        });

        for _assignment in assignments {
          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
        }
      };
      let mut pattern_visitor = PatternVisitor::new(self, cb);
      pattern_visitor.visit_pat(&pat, catch_clause);
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
    _parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      for_in_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);

    use swc_ecma_ast::VarDeclOrPat;

    match &for_in_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        self.visit_var_decl(var_decl, for_in_stmt);

        let cb = |sv: &mut ScopeVisitor,
                  ident: &swc_ecma_ast::Ident,
                  _assignments: &[Box<swc_ecma_ast::Expr>]| {
          let mut scope = sv.get_current_scope();

          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
        };
        let mut pattern_visitor = PatternVisitor::new(self, cb);
        pattern_visitor.visit_pat(&var_decl.decls[0].name, for_in_stmt);
      }
      VarDeclOrPat::Pat(pat) => {
        let cb = |sv: &mut ScopeVisitor,
                  ident: &swc_ecma_ast::Ident,
                  assignments: &[Box<swc_ecma_ast::Expr>]| {
          let mut scope = sv.get_current_scope();

          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
          for _assignment in assignments {
            // eprintln!("assignment {:#?}", assignment);
            let ref_ = Reference {
              name: ident.sym.to_string(),
              kind: ReferenceKind::Write,
            };
            scope.add_reference(ref_);
          }
        };
        let mut pattern_visitor = PatternVisitor::new(self, cb);
        pattern_visitor.visit_pat(&pat, for_in_stmt);
      }
    };

    self.visit_expr(&*for_in_stmt.right, for_in_stmt);
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
    _parent: &dyn Node,
  ) {
    let loop_scope = Scope::new(
      ScopeKind::Loop,
      for_of_stmt.span,
      Some(self.get_current_scope()),
    );
    self.enter_scope(&loop_scope);

    use swc_ecma_ast::VarDeclOrPat;

    match &for_of_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        self.visit_var_decl(var_decl, for_of_stmt);

        let cb = |sv: &mut ScopeVisitor,
                  ident: &swc_ecma_ast::Ident,
                  _assignments: &[Box<swc_ecma_ast::Expr>]| {
          let mut scope = sv.get_current_scope();

          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
        };
        let mut pattern_visitor = PatternVisitor::new(self, cb);
        pattern_visitor.visit_pat(&var_decl.decls[0].name, for_of_stmt);
      }
      VarDeclOrPat::Pat(pat) => {
        let cb = |sv: &mut ScopeVisitor,
                  ident: &swc_ecma_ast::Ident,
                  assignments: &[Box<swc_ecma_ast::Expr>]| {
          let mut scope = sv.get_current_scope();

          let ref_ = Reference {
            name: ident.sym.to_string(),
            kind: ReferenceKind::Write,
          };
          scope.add_reference(ref_);
          for _assignment in assignments {
            // eprintln!("assignment {:#?}", assignment);
            let ref_ = Reference {
              name: ident.sym.to_string(),
              kind: ReferenceKind::Write,
            };
            scope.add_reference(ref_);
          }
        };
        let mut pattern_visitor = PatternVisitor::new(self, cb);
        pattern_visitor.visit_pat(&pat, for_of_stmt);
      }
    };

    self.visit_expr(&*for_of_stmt.right, for_of_stmt);
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
    let bindings = catch_scope.get_bindings();
    let catch_clause_e = &bindings[0];
    assert_eq!(catch_clause_e.name, "e");
    assert_eq!(catch_clause_e.kind, BindingKind::CatchClause);
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
    let switch_bindings = switch_scope.get_bindings();
    assert_eq!(&switch_bindings[0].name, "a");
    assert_eq!(&switch_bindings[1].name, "defaultVal");
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
    assert_eq!(for_scope.get_bindings()[0].name, "i");

    let for_in_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(for_in_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(for_in_scope.child_scope_count(), 0);
    assert_eq!(for_in_scope.get_bindings()[0].name, "i");

    let for_of_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(for_of_scope.get_kind(), ScopeKind::Loop);
    assert_eq!(for_of_scope.child_scope_count(), 0);
    assert_eq!(for_of_scope.get_bindings()[0].name, "i");

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
    assert_eq!(obj_method_scope.get_bindings()[0].name, "e");

    let obj_nested_method_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(obj_nested_method_scope.get_kind(), ScopeKind::Function);
    assert_eq!(obj_nested_method_scope.get_bindings()[0].name, "f");

    let obj_getter_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(obj_getter_scope.get_kind(), ScopeKind::Function);
    assert_eq!(obj_getter_scope.get_bindings()[0].name, "g");

    let obj_setter_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(obj_setter_scope.get_kind(), ScopeKind::Function);
    assert_eq!(obj_setter_scope.get_bindings()[0].name, "h");
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
    let array_fn_bindings = array_fn_scope.get_bindings();
    assert_eq!(array_fn_bindings.len(), 1);
    assert_eq!(array_fn_bindings[0].name, "a");

    let array_arrow_scope = module_scope.get_child_scope(1).unwrap();
    assert_eq!(array_arrow_scope.get_kind(), ScopeKind::Block);
    let array_arrow_bindings = array_arrow_scope.get_bindings();
    assert_eq!(array_arrow_bindings.len(), 1);
    assert_eq!(array_arrow_bindings[0].name, "b");

    let array_nested_fn_scope = module_scope.get_child_scope(2).unwrap();
    assert_eq!(array_nested_fn_scope.get_kind(), ScopeKind::Function);
    let array_nested_fn_bindings = array_nested_fn_scope.get_bindings();
    assert_eq!(array_nested_fn_bindings.len(), 1);
    assert_eq!(array_nested_fn_bindings[0].name, "c");

    let array_object_method_scope = module_scope.get_child_scope(3).unwrap();
    assert_eq!(array_object_method_scope.get_kind(), ScopeKind::Function);
    let array_object_method_bindings = array_object_method_scope.get_bindings();
    assert_eq!(array_object_method_bindings.len(), 1);
    assert_eq!(array_object_method_bindings[0].name, "d");
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
    let bindings = module_scope.get_bindings();

    let default_export = &bindings[0];
    assert_eq!(default_export.name, "defaultExport1");
    assert_eq!(default_export.kind, BindingKind::Import);

    let namespaced1 = &bindings[1];
    assert_eq!(namespaced1.name, "namespaced1");
    assert_eq!(namespaced1.kind, BindingKind::Import);

    let export1 = &bindings[2];
    assert_eq!(export1.name, "export1");
    assert_eq!(export1.kind, BindingKind::Import);

    assert!(bindings.iter().find(|b| b.name == "export2").is_none());
    assert!(bindings.iter().any(|b| b.name == "alias1"));
    assert!(bindings.iter().any(|b| b.name == "export3"));
    assert!(bindings.iter().any(|b| b.name == "export4"));
    assert!(bindings.iter().any(|b| b.name == "export5"));
    assert!(bindings.iter().find(|b| b.name == "export6").is_none());
    assert!(bindings.iter().any(|b| b.name == "alias2"));
    assert!(bindings.iter().any(|b| b.name == "defaultExport2"));
    assert!(bindings.iter().any(|b| b.name == "export7"));
    assert!(bindings.iter().any(|b| b.name == "defaultExport3"));
    assert!(bindings.iter().any(|b| b.name == "namespaced2"));
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
function getPerson({username="disizali", name, family}, ...restParam) {
  try {
    throw 'TryAgain';
  } catch(e) {}
}
try{} catch({message}){};
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    let module_scope_bindings = module_scope.get_bindings();
    assert_eq!(module_scope_bindings.len(), 23);

    assert_eq!(module_scope_bindings[0].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[0].name, "a");

    assert_eq!(module_scope_bindings[1].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[1].name, "a");

    assert_eq!(module_scope_bindings[2].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[2].name, "b");

    assert_eq!(module_scope_bindings[3].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[3].name, "a");

    assert_eq!(module_scope_bindings[4].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[4].name, "b");

    assert_eq!(module_scope_bindings[5].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[5].name, "c");

    assert_eq!(module_scope_bindings[6].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[6].name, "d");

    assert_eq!(module_scope_bindings[7].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[7].name, "e");

    assert_eq!(module_scope_bindings[8].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[8].name, "f");

    assert_eq!(module_scope_bindings[9].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[9].name, "g");

    assert_eq!(module_scope_bindings[10].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[10].name, "a");

    assert_eq!(module_scope_bindings[11].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[11].name, "h");

    assert_eq!(module_scope_bindings[12].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[12].name, "i");

    assert_eq!(module_scope_bindings[13].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[13].name, "j");

    assert_eq!(module_scope_bindings[14].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[14].name, "a");

    assert_eq!(module_scope_bindings[15].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[15].name, "b");

    assert_eq!(module_scope_bindings[16].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[16].name, "k");

    assert_eq!(module_scope_bindings[17].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[17].name, "l");

    assert_eq!(module_scope_bindings[18].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[18].name, "m");

    assert_eq!(module_scope_bindings[19].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[19].name, "n");

    assert_eq!(module_scope_bindings[20].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[20].name, "o");

    assert_eq!(module_scope_bindings[21].kind, BindingKind::Const);
    assert_eq!(module_scope_bindings[21].name, "p");

    assert_eq!(module_scope_bindings[22].kind, BindingKind::Function);
    assert_eq!(module_scope_bindings[22].name, "getPerson");

    let function_scope = module_scope.get_child_scope(0).unwrap();
    let function_scope_bindings = function_scope.get_bindings();

    assert_eq!(function_scope_bindings.len(), 4);
    assert_eq!(function_scope_bindings[0].name, "username");
    assert_eq!(function_scope_bindings[1].name, "name");
    assert_eq!(function_scope_bindings[2].name, "family");
    assert_eq!(function_scope_bindings[3].name, "restParam");

    let function_catch_scope = function_scope.get_child_scope(-1).unwrap();
    let function_catch_bindings = function_catch_scope.get_bindings();
    assert_eq!(function_catch_bindings.len(), 1);
    assert_eq!(function_catch_bindings[0].name, "e");

    let catch_scope = module_scope.get_child_scope(-1).unwrap();
    let catch_bindings = catch_scope.get_bindings();
    assert_eq!(catch_bindings.len(), 1);
    assert_eq!(catch_bindings[0].name, "message");
  }

  #[test]
  fn references1() {
    let source_code = r#"
let a = 0;
try {}
catch ({ data = "asdf" }) {}
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    let mod_bindings = module_scope.get_bindings();
    assert_eq!(mod_bindings.len(), 1);
    assert_eq!(mod_bindings[0].name, "a");
    let refs = module_scope.get_references();
    assert_eq!(refs.len(), 1);
    let a_ref = &refs[0];
    assert_eq!(a_ref.name, "a");
    assert_eq!(a_ref.kind, ReferenceKind::Write);

    let catch_scope = module_scope.get_child_scope(-1).unwrap();
    let catch_bindings = catch_scope.get_bindings();
    assert_eq!(catch_bindings.len(), 1);
    assert_eq!(catch_bindings[0].name, "data");
    let catch_refs = catch_scope.get_references();
    assert_eq!(catch_refs.len(), 1);
    assert_eq!(catch_refs[0].name, "data");
    assert_eq!(catch_refs[0].kind, ReferenceKind::Write);
  }

  #[test]
  fn references2() {
    let source_code = r#"
let a = 0;
function b() {
  let c = a;
  let d = 1;
  c += d;
  c = 0;
}
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    let mod_bindings = module_scope.get_bindings();
    assert_eq!(mod_bindings.len(), 2);
    assert_eq!(mod_bindings[0].name, "a");
    assert_eq!(mod_bindings[1].name, "b");

    let mod_refs = module_scope.get_references();
    assert_eq!(mod_refs.len(), 1); // a
    let a_ref = &mod_refs[0];
    assert_eq!(a_ref.name, "a");
    assert_eq!(a_ref.kind, ReferenceKind::Write);

    let fn_scope = module_scope.get_child_scope(0).unwrap();
    let fn_refs = fn_scope.get_references();
    let fn_bindings = fn_scope.get_bindings();
    assert_eq!(fn_bindings.len(), 2); // c
    assert_eq!(fn_bindings[0].name, "c");
    assert_eq!(fn_bindings[1].name, "d");

    assert_eq!(fn_refs.len(), 6);
    let c_ref = &fn_refs[0];
    assert_eq!(c_ref.name, "c");
    assert_eq!(a_ref.kind, ReferenceKind::Write);
    let a_ref = &fn_refs[1];
    assert_eq!(a_ref.name, "a");
    assert_eq!(a_ref.kind, ReferenceKind::Read);
    let d_ref1 = &fn_refs[2];
    assert_eq!(d_ref1.name, "d");
    assert_eq!(d_ref1.kind, ReferenceKind::Write);
    let c_ref2 = &fn_refs[3];
    assert_eq!(c_ref2.name, "c");
    assert_eq!(c_ref2.kind, ReferenceKind::ReadWrite);
    let d_ref2 = &fn_refs[4];
    assert_eq!(d_ref2.name, "d");
    assert_eq!(d_ref2.kind, ReferenceKind::Read);
    let c_ref3 = &fn_refs[5];
    assert_eq!(c_ref3.name, "c");
    assert_eq!(c_ref3.kind, ReferenceKind::Write);
  }

  #[test]
  fn references3() {
    let source_code = r#"
for (const foo in bar) {}

for (const fizz of buzz) {}
"#;
    let root_scope = test_scopes(source_code);
    let module_scope = root_scope.get_child_scope(0).unwrap();
    let mod_bindings = module_scope.get_bindings();
    assert_eq!(mod_bindings.len(), 0);

    let for_in_scope = module_scope.get_child_scope(0).unwrap();
    let for_in_bindings = for_in_scope.get_bindings();
    assert_eq!(for_in_bindings.len(), 1); // foo
    assert_eq!(for_in_bindings[0].name, "foo");

    let for_in_refs = for_in_scope.get_references();
    eprintln!("for in refs {:#?}", for_in_refs);
    assert_eq!(for_in_refs.len(), 2); // foo, bar
    assert_eq!(for_in_refs[0].name, "foo");
    assert_eq!(for_in_refs[0].kind, ReferenceKind::Write);
    assert_eq!(for_in_refs[1].name, "bar");
    assert_eq!(for_in_refs[1].kind, ReferenceKind::Read);

    let for_of_scope = module_scope.get_child_scope(1).unwrap();
    let for_of_bindings = for_of_scope.get_bindings();
    assert_eq!(for_of_bindings.len(), 1); // c
    assert_eq!(for_of_bindings[0].name, "fizz");

    let for_of_refs = for_of_scope.get_references();
    assert_eq!(for_of_refs.len(), 2); // fizz, buzz
    assert_eq!(for_of_refs[0].name, "fizz");
    assert_eq!(for_of_refs[0].kind, ReferenceKind::Write);
    assert_eq!(for_of_refs[1].name, "buzz");
    assert_eq!(for_of_refs[1].kind, ReferenceKind::Read);
  }
}
