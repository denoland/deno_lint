// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use swc_common::Span;
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

lazy_static! {
  static ref NEXT_ID: AtomicU32 = AtomicU32::new(0);
}

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

#[derive(Clone, Debug, PartialEq)]
pub enum ScopeKind {
  Program,
  Module,
  Function,
  Block,
  Loop,
  Class,
}

#[derive(Clone, Debug)]
pub struct Scope {
  pub kind: ScopeKind,
  pub id: u32,
  pub parent_id: Option<u32>,
  pub span: Span,
  pub child_scopes: Vec<u32>,
  pub bindings: HashMap<String, Binding>,
}

impl Scope {
  pub fn new(kind: ScopeKind, span: Span, parent_id: Option<u32>) -> Self {
    Self {
      kind,
      span,
      parent_id,
      id: next_id(),
      child_scopes: vec![],
      bindings: HashMap::new(),
    }
  }

  pub fn add_binding(&mut self, binding: Binding) {
    self.bindings.insert(binding.name.to_string(), binding);
  }

  pub fn get_binding(&self, name: &str) -> Option<&Binding> {
    self.bindings.get(name)
  }
}

#[derive(Debug)]
pub struct ScopeManager {
  pub scopes: HashMap<u32, Scope>,
  pub scope_stack: Vec<u32>,
}

impl ScopeManager {
  pub fn new() -> Self {
    Self {
      scopes: HashMap::new(),
      scope_stack: vec![],
    }
  }

  pub fn set_root_scope(&mut self, scope: Scope) {
    self.scope_stack.push(scope.id);
    self.scopes.insert(scope.id, scope);
  }

  pub fn enter_scope(&mut self, scope: Scope) {
    let current_scope = self.get_current_scope_mut();
    current_scope.child_scopes.push(scope.id);
    self.scope_stack.push(scope.id);
    self.scopes.insert(scope.id, scope);
  }

  pub fn exit_scope(&mut self) {
    self.scope_stack.pop();
  }

  pub fn get_root_scope(&mut self) -> &mut Scope {
    self
      .get_scope_mut(*self.scope_stack.first().unwrap())
      .unwrap()
  }

  pub fn get_current_scope_id(&self) -> u32 {
    assert!(!self.scope_stack.is_empty());
    *self.scope_stack.last().unwrap()
  }

  pub fn get_current_scope(&self) -> &Scope {
    self.get_scope(self.get_current_scope_id()).unwrap()
  }

  pub fn get_current_scope_mut(&mut self) -> &mut Scope {
    self.get_scope_mut(self.get_current_scope_id()).unwrap()
  }

  pub fn get_scope(&self, id: u32) -> Option<&Scope> {
    self.scopes.get(&id)
  }

  pub fn get_scope_mut(&mut self, id: u32) -> Option<&mut Scope> {
    self.scopes.get_mut(&id)
  }

  pub fn add_binding(&mut self, binding: Binding) {
    let current_scope = self.get_current_scope_mut();
    current_scope.add_binding(binding);
  }

  pub fn get_parent_scope(&self, scope: &Scope) {}

  pub fn get_binding<'a>(
    &'a self,
    s: &'a Scope,
    name: &str,
  ) -> Option<&'a Binding> {
    let mut scope = s;

    loop {
      if let Some(binding) = scope.get_binding(name) {
        return Some(binding);
      }
      if let Some(parent_id) = scope.parent_id {
        scope = self.get_scope(parent_id).unwrap();
      } else {
        break;
      }
    }

    None
  }

  pub fn get_scope_for_span(&self, span: Span) -> &Scope {
    let mut current_scope: Option<&Scope> = None;

    for (_id, scope) in self.scopes.iter() {
      if scope.span.contains(span) {
        match current_scope {
          Some(s) => {
            // If currently found scope span fully encloses
            // iterated span then it's a child scope.
            if s.span.contains(scope.span) {
              current_scope = Some(scope);
            }
          }
          None => {
            current_scope = Some(scope);
          }
        }
      }
    }

    assert!(current_scope.is_some());
    &current_scope.unwrap()
  }
}

pub struct ScopeVisitor {
  pub scope_manager: ScopeManager,
}

fn next_id() -> u32 {
  NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

impl ScopeVisitor {
  pub fn new() -> Self {
    Self {
      scope_manager: ScopeManager::new(),
    }
  }

  pub fn consume(self) -> ScopeManager {
    self.scope_manager
  }
}

impl Visit for ScopeVisitor {
  fn visit_module(&mut self, module: &swc_ecma_ast::Module, parent: &dyn Node) {
    let program_scope = Scope::new(ScopeKind::Program, module.span, None);

    self.scope_manager.set_root_scope(program_scope);

    let module_scope = Scope::new(
      ScopeKind::Module,
      module.span,
      Some(self.scope_manager.get_current_scope_id()),
    );

    self.scope_manager.enter_scope(module_scope);
    swc_ecma_visit::visit_module(self, module, parent);

    self.scope_manager.exit_scope();

    // program scope is left on stack
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
    self.scope_manager.add_binding(fn_binding);

    let fn_scope = Scope::new(
      ScopeKind::Function,
      fn_decl.function.span,
      Some(self.scope_manager.get_current_scope_id()),
    );
    self.scope_manager.enter_scope(fn_scope);

    swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);
    self.scope_manager.exit_scope();
  }

  fn visit_function(
    &mut self,
    function: &swc_ecma_ast::Function,
    parent: &dyn Node,
  ) {
    for param in &function.params {
      let name = match &param.pat {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => todo!(),
      };
      let param_binding = Binding {
        kind: BindingKind::Param,
        name,
      };
      self.scope_manager.add_binding(param_binding);
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

      self.scope_manager.add_binding(Binding {
        kind: var_kind.clone(),
        name,
      })
    }
    swc_ecma_visit::visit_var_decl(self, var_decl, parent);
  }

  fn visit_block_stmt(
    &mut self,
    block_stmt: &swc_ecma_ast::BlockStmt,
    parent: &dyn Node,
  ) {
    let block_scope = Scope::new(
      ScopeKind::Block,
      block_stmt.span,
      Some(self.scope_manager.get_current_scope_id()),
    );
    self.scope_manager.enter_scope(block_scope);

    swc_ecma_visit::visit_block_stmt(self, block_stmt, parent);
    self.scope_manager.exit_scope();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::swc_util;
  use crate::swc_util::AstParser;
  use crate::swc_util::SwcDiagnosticBuffer;

  #[test]
  fn scopes() {
    let ast_parser = AstParser::new();
    let syntax = swc_util::get_default_ts_config();

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
}"#;

    let r: Result<ScopeManager, SwcDiagnosticBuffer> = ast_parser.parse_module(
      "file_name.ts",
      syntax,
      source_code,
      |parse_result, comments| {
        let module = parse_result?;
        let mut scope_visitor = ScopeVisitor::new();
        scope_visitor.visit_module(&module, &module);
        let root_scope = scope_visitor.consume();
        Ok(root_scope)
      },
    );
    assert!(r.is_ok());
    let mut scope_manager = r.unwrap();

    let root_scope = scope_manager.get_root_scope();
    assert_eq!(root_scope.kind, ScopeKind::Program);
    assert_eq!(root_scope.child_scopes.len(), 1);

    let module_scope_id = *root_scope.child_scopes.first().unwrap();
    let module_scope = scope_manager.get_scope(module_scope_id).unwrap();
    assert_eq!(module_scope.kind, ScopeKind::Module);
    assert_eq!(module_scope.child_scopes.len(), 1);

    let fn_scope_id = *module_scope.child_scopes.first().unwrap();
    let fn_scope = scope_manager.get_scope(fn_scope_id).unwrap();
    assert_eq!(fn_scope.kind, ScopeKind::Function);
    assert_eq!(fn_scope.child_scopes.len(), 1);

    let block_scope_id = *fn_scope.child_scopes.first().unwrap();
    let block_scope = scope_manager.get_scope(block_scope_id).unwrap();
    assert_eq!(block_scope.kind, ScopeKind::Block);
  }
}
