// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use swc_common::Span;
use swc_ecma_ast::ObjectPatProp;
use swc_ecma_ast::Pat;
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
  Class,
  CatchClause,
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
  Switch,
  With,
  Catch,
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

  pub fn exit_scope(&mut self, scope_id: u32) {
    assert!(self.scope_stack.len() > 1);
    let exit_id = self.scope_stack.pop().unwrap();
    assert_eq!(exit_id, scope_id);
  }

  pub fn get_root_scope(&self) -> &Scope {
    self.get_scope(*self.scope_stack.first().unwrap()).unwrap()
  }

  pub fn get_current_scope_id(&self) -> u32 {
    assert!(!self.scope_stack.is_empty());
    *self.scope_stack.last().unwrap()
  }

  #[allow(unused)]
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

  #[allow(unused)]
  pub fn get_parent_scope(&self, scope: &Scope) -> Option<&Scope> {
    if let Some(parent_scope_id) = scope.parent_id {
      return self.get_scope(parent_scope_id);
    }
    None
  }

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

  fn find_scope(&self, child_scopes: &[u32], span: Span) -> Option<u32> {
    for scope_id in child_scopes {
      let child_scope = self.get_scope(*scope_id).unwrap();
      if child_scope.span.contains(span) {
        let found_scope_id =
          match self.find_scope(&child_scope.child_scopes, span) {
            Some(id) => id,
            None => child_scope.id,
          };
        return Some(found_scope_id);
      }
    }

    None
  }

  pub fn get_scope_for_span(&self, span: Span) -> &Scope {
    let root_scope = self.get_root_scope();

    let scope_id = match self.find_scope(&root_scope.child_scopes, span) {
      Some(id) => id,
      None => root_scope.id,
    };

    let scope = self.get_scope(scope_id).unwrap();
    &scope
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
    let module_scope_id = module_scope.id;
    self.scope_manager.enter_scope(module_scope);

    swc_ecma_visit::visit_module(self, module, parent);

    self.scope_manager.exit_scope(module_scope_id);
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
    let fn_scope_id = fn_scope.id;
    self.scope_manager.enter_scope(fn_scope);

    swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);

    self.scope_manager.exit_scope(fn_scope_id);
  }

  fn visit_class_decl(
    &mut self,
    class_decl: &swc_ecma_ast::ClassDecl,
    parent: &dyn Node,
  ) {
    let class_binding = Binding {
      kind: BindingKind::Class,
      name: class_decl.ident.sym.to_string(),
    };
    self.scope_manager.add_binding(class_binding);

    let class_scope = Scope::new(
      ScopeKind::Class,
      class_decl.class.span,
      Some(self.scope_manager.get_current_scope_id()),
    );
    let class_scope_id = class_scope.id;
    self.scope_manager.enter_scope(class_scope);

    swc_ecma_visit::visit_class_decl(self, class_decl, parent);

    self.scope_manager.exit_scope(class_scope_id);
  }

  fn visit_function(
    &mut self,
    function: &swc_ecma_ast::Function,
    _parent: &dyn Node,
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

    // Not calling `swc_ecma_visit::visit_function` but instead
    // directly visiting body elements - otherwise additional
    // block scope will be created which is undesirable
    if let Some(body) = &function.body {
      for stmt in &body.stmts {
        swc_ecma_visit::visit_stmt(self, stmt, body);
      }
    }
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
      Some(self.scope_manager.get_current_scope_id()),
    );
    let with_scope_id = with_scope.id;
    self.scope_manager.enter_scope(with_scope);
    swc_ecma_visit::visit_stmt(self, &*with_stmt.body, with_stmt);
    self.scope_manager.exit_scope(with_scope_id);
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
      Some(self.scope_manager.get_current_scope_id()),
    );
    let switch_scope_id = switch_scope.id;
    self.scope_manager.enter_scope(switch_scope);
    for case in &switch_stmt.cases {
      swc_ecma_visit::visit_switch_case(self, case, switch_stmt);
    }
    self.scope_manager.exit_scope(switch_scope_id);
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
    let block_scope_id = block_scope.id;
    self.scope_manager.enter_scope(block_scope);

    swc_ecma_visit::visit_block_stmt(self, block_stmt, parent);
    self.scope_manager.exit_scope(block_scope_id);
  }

  fn visit_catch_clause(
    &mut self,
    catch_clause: &swc_ecma_ast::CatchClause,
    _parent: &dyn Node,
  ) {
    let catch_scope = Scope::new(
      ScopeKind::Catch,
      catch_clause.span,
      Some(self.scope_manager.get_current_scope_id()),
    );
    let catch_scope_id = catch_scope.id;
    self.scope_manager.enter_scope(catch_scope);

    if let Some(pat) = &catch_clause.param {
      match pat {
        Pat::Ident(ident) => {
          self.scope_manager.add_binding(Binding {
            kind: BindingKind::CatchClause,
            name: ident.sym.to_string(),
          });
        }
        Pat::Object(object_pat) => {
          for prop in &object_pat.props {
            match prop {
              ObjectPatProp::Assign(assign_pat_prop) => {
                // TODO(bartlomieju): handle default value
                self.scope_manager.add_binding(Binding {
                  kind: BindingKind::CatchClause,
                  name: assign_pat_prop.key.sym.to_string(),
                });
              }
              _ => todo!(),
            }
          }
        }
        _ => todo!(),
      };
    }

    // Not calling `swc_ecma_visit::visit_class` but instead
    // directly visiting body elements - otherwise additional
    // block scope will be created which is undesirable
    for stmt in &catch_clause.body.stmts {
      swc_ecma_visit::visit_stmt(self, stmt, &catch_clause.body);
    }

    self.scope_manager.exit_scope(catch_scope_id);
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

    let r: Result<ScopeManager, SwcDiagnosticBuffer> = ast_parser.parse_module(
      "file_name.ts",
      syntax,
      source_code,
      |parse_result, _comments| {
        let module = parse_result?;
        let mut scope_visitor = ScopeVisitor::new();
        scope_visitor.visit_module(&module, &module);
        let root_scope = scope_visitor.consume();
        Ok(root_scope)
      },
    );
    assert!(r.is_ok());
    let scope_manager = r.unwrap();

    let root_scope = scope_manager.get_root_scope();
    assert_eq!(root_scope.kind, ScopeKind::Program);
    assert_eq!(root_scope.child_scopes.len(), 1);

    let module_scope_id = *root_scope.child_scopes.first().unwrap();
    let module_scope = scope_manager.get_scope(module_scope_id).unwrap();
    assert_eq!(module_scope.kind, ScopeKind::Module);
    assert_eq!(module_scope.child_scopes.len(), 4);

    let fn_scope_id = *module_scope.child_scopes.first().unwrap();
    let fn_scope = scope_manager.get_scope(fn_scope_id).unwrap();
    assert_eq!(fn_scope.kind, ScopeKind::Function);
    assert_eq!(fn_scope.child_scopes.len(), 1);

    let block_scope_id = *fn_scope.child_scopes.first().unwrap();
    let block_scope = scope_manager.get_scope(block_scope_id).unwrap();
    assert_eq!(block_scope.kind, ScopeKind::Block);
    assert_eq!(block_scope.child_scopes.len(), 0);

    let class_scope_id = *module_scope.child_scopes.get(1).unwrap();
    let class_scope = scope_manager.get_scope(class_scope_id).unwrap();
    assert_eq!(class_scope.kind, ScopeKind::Class);
    assert_eq!(class_scope.child_scopes.len(), 0);

    let catch_scope_id = *module_scope.child_scopes.get(3).unwrap();
    let catch_scope = scope_manager.get_scope(catch_scope_id).unwrap();
    assert_eq!(catch_scope.kind, ScopeKind::Catch);
    assert_eq!(catch_scope.child_scopes.len(), 0);
    let catch_clause_e = catch_scope.get_binding("e").unwrap();
    assert_eq!(catch_clause_e.kind, BindingKind::CatchClause);
  }

  #[test]
  fn switch_scope() {
    let ast_parser = AstParser::new();
    let syntax = swc_util::get_default_ts_config();

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

    let r: Result<ScopeManager, SwcDiagnosticBuffer> = ast_parser.parse_module(
      "file_name.ts",
      syntax,
      source_code,
      |parse_result, _comments| {
        let module = parse_result?;
        let mut scope_visitor = ScopeVisitor::new();
        scope_visitor.visit_module(&module, &module);
        let root_scope = scope_visitor.consume();
        Ok(root_scope)
      },
    );
    assert!(r.is_ok());
    let scope_manager = r.unwrap();

    let root_scope = scope_manager.get_root_scope();
    assert_eq!(root_scope.kind, ScopeKind::Program);
    assert_eq!(root_scope.child_scopes.len(), 1);

    let module_scope_id = *root_scope.child_scopes.first().unwrap();
    let module_scope = scope_manager.get_scope(module_scope_id).unwrap();
    assert_eq!(module_scope.kind, ScopeKind::Module);
    assert_eq!(module_scope.child_scopes.len(), 1);

    let switch_scope_id = *module_scope.child_scopes.first().unwrap();
    let switch_scope = scope_manager.get_scope(switch_scope_id).unwrap();
    assert_eq!(switch_scope.kind, ScopeKind::Switch);
    assert_eq!(switch_scope.child_scopes.len(), 0);
    assert!(switch_scope.get_binding("a").is_some());
    assert!(switch_scope.get_binding("defaultVal").is_some());
  }
}
