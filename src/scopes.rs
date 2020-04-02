// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use swc_ecma_ast::AssignExpr;
use swc_ecma_ast::Decl;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprStmt;
use swc_ecma_ast::Module;
use swc_ecma_ast::ModuleItem;
use swc_ecma_ast::Pat;
use swc_ecma_ast::Stmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

#[derive(Clone, Debug, PartialEq)]
pub enum BindingKind {
  Var,
  Const,
  Let,
  Function,
  Param,
}

#[derive(Clone, Debug)]
pub struct Binding {
  pub kind: BindingKind,
  pub name: String,
}

#[derive(Clone, Debug)]
pub enum ScopeKind {
  Root,
  Module,
  Function,
  Block,
  Loop,
  Class,
}

#[derive(Clone, Debug)]
pub struct Scope {
  pub kind: ScopeKind,
  pub bindings: HashMap<String, Binding>,
}

impl Scope {
  pub fn new(kind: ScopeKind) -> Self {
    Self {
      kind,
      bindings: HashMap::new(),
    }
  }
}

#[derive(Debug)]
pub struct ScopeVisitor {
  pub scope_stack: Vec<Scope>,
}

impl ScopeVisitor {
  pub fn default() -> Self {
    let root_scope = Scope::new(ScopeKind::Root);

    Self {
      scope_stack: vec![root_scope],
    }
  }

  pub fn get_current_scope(&mut self) -> &mut Scope {
    self.scope_stack.last_mut().expect("Scope stack empty")
  }

  pub fn add_binding(&mut self, binding: Binding) {
    let current_scope = self.get_current_scope();
    current_scope
      .bindings
      .insert(binding.name.to_string(), binding);
  }

  pub fn get_binding(&self, name: &str) -> Option<&Binding> {
    for scope in self.scope_stack.iter().rev() {
      if let Some(binding) = scope.bindings.get(name) {
        return Some(binding);
      }
    }

    None
  }
}

impl Visit for ScopeVisitor {
  fn visit_module(&mut self, module: &swc_ecma_ast::Module, parent: &dyn Node) {
    let module_scope = Scope::new(ScopeKind::Module);
    self.scope_stack.push(module_scope);
    swc_ecma_visit::visit_module(self, module, parent);
    dbg!(&self);
    self.scope_stack.pop();
  }

  fn visit_fn_decl(
    &mut self,
    fn_decl: &swc_ecma_ast::FnDecl,
    parent: &dyn Node,
  ) {
    let name = fn_decl.ident.sym.to_string();
    let fn_binding = Binding {
      kind: BindingKind::Function,
      name,
    };
    self.add_binding(fn_binding);
    let fn_scope = Scope::new(ScopeKind::Function);
    self.scope_stack.push(fn_scope);
    swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);
    dbg!(&self);
    self.scope_stack.pop();
  }

  fn visit_function(
    &mut self,
    function: &swc_ecma_ast::Function,
    parent: &dyn Node,
  ) {
    for param in &function.params {
      let name = match param {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => todo!(),
      };
      let param_binding = Binding {
        kind: BindingKind::Param,
        name,
      };
      self.add_binding(param_binding);
    }
    swc_ecma_visit::visit_function(self, function, parent);
  }

  fn visit_var_decl(
    &mut self,
    var_decl: &swc_ecma_ast::VarDecl,
    parent: &dyn Node,
  ) {
    use swc_ecma_ast::VarDeclKind;

    let var_kind = match &var_decl.kind {
      VarDeclKind::Var => BindingKind::Var,
      VarDeclKind::Let => BindingKind::Let,
      VarDeclKind::Const => BindingKind::Const,
    };

    for decl in &var_decl.decls {
      let name = match &decl.name {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => todo!(),
      };

      self.add_binding(Binding {
        kind: var_kind.clone(),
        name,
      })
    }
    swc_ecma_visit::visit_var_decl(self, var_decl, parent);
  }
}
