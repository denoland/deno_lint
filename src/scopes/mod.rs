// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use crate::ast_node::AstNode;
use crate::ast_node::AstNodeKind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use swc_ecma_ast::Decl;
use swc_ecma_ast::Module;
use swc_ecma_ast::ModuleItem;
use swc_ecma_ast::Pat;
use swc_ecma_ast::Stmt;

pub struct LintContext {
  pub root_scope: Rc<RefCell<Scope>>,
  pub scope_stack: Vec<Rc<RefCell<Scope>>>,
}

impl LintContext {
  pub fn new(node: Box<AstNode>) -> Self {
    Self {
      root_scope: Rc::new(RefCell::new(Scope::new(node, ScopeKind::Root))),
      scope_stack: vec![],
    }
  }

  pub fn walk(&mut self, node: AstNode, transforms: &[Box<dyn LintTransform>]) {
    let _current_scope = self.get_current_scope();

    // TODO: figure out if we're entering new scope
    // if let Some(new_scope) = node.build_scope() {
    //     self.scope_stack.push(new_scope.clone());
    // }

    let _current_scope = self.get_current_scope();
    eprintln!("walk {:#?}, {}", node.kind(), transforms.len());
    // run all enter transformations for this node
    for transform in transforms {
      transform.enter(self, node.clone())
    }

    // walk children
    let children_nodes = node.get_children();
    for child in &children_nodes {
      self.walk(child.clone(), transforms)
    }

    // run all exit transformations for this node
    for transform in transforms {
      transform.exit(self, node.clone())
    }

    // TODO: pop scope if needed
  }

  pub fn walk_module(
    &mut self,
    module: Module,
    transforms: &[Box<dyn LintTransform>],
  ) {
    assert!(self.scope_stack.is_empty());

    let module_node = Box::new(AstNode::Module(module.clone()));
    let module_scope = Rc::new(RefCell::new(Scope::new(
      module_node.clone(),
      ScopeKind::Module,
    )));

    let current_scope = self.get_current_scope();
    {
      let mut mut_scope = current_scope.borrow_mut();
      mut_scope.add_child_scope(*module_node, module_scope.clone());
    }

    // Entering module scope
    self.scope_stack.push(module_scope);

    for module_item in &module.body {
      match module_item {
        ModuleItem::ModuleDecl(module_decl) => {}
        ModuleItem::Stmt(stmt) => self.walk_statement(stmt.clone()),
      }
    }

    // Exiting module scope
    self.scope_stack.pop();
    eprintln!("{:#?}", self.get_current_scope());
    eprintln!("{:#?}", self.scope_stack);
  }

  fn walk_statement(&mut self, stmt: Stmt) {
    match stmt {
      // Stmt::Block(block_stmt) => self.walk_block_stmt(block_stmt),
      // Stmt::Empty(empty_stmt) => self.walk_empty_stmt(empty_stmt),
      // Stmt::Debugger(debugger_stmt) => self.walk_debugger_stmt(debugger_stmt),
      // Stmt::With(with_stmt) => self.walk_with_stmt(with_stmt),
      // Stmt::Return(return_stmt) => self.walk_return_stmt(return_stmt),
      // Stmt::Labeled(labeled_stmt) => self.walk_labeled_stmt(labeled_stmt),
      // Stmt::Break(break_stmt) => self.walk_break_stmt(break_stmt),
      // Stmt::Continue(continue_stmt) => self.walk_continue_stmt(continue_stmt),
      // Stmt::If(if_stmt) => self.walk_if_stmt(if_stmt),
      // Stmt::Switch(switch_stmt) => self.walk_switch_stmt(switch_stmt),
      // Stmt::Throw(throw_stmt) => self.walk_throw_stmt(throw_stmt),
      // Stmt::Try(try_stmt) => self.walk_try_stmt(try_stmt),
      // Stmt::While(while_stmt) => self.walk_while_stmt(while_stmt),
      // Stmt::DoWhile(do_while_stmt) => self.walk_do_while_stmt(do_while_stmt),
      // Stmt::For(for_stmt) => self.walk_for_stmt(for_stmt),
      // Stmt::ForIn(for_in_stmt) => self.walk_for_in_stmt(for_in_stmt),
      // Stmt::ForOf(for_of_stmt) => self.walk_for_of_stmt(for_of_stmt),
      Stmt::Decl(decl) => self.walk_decl(decl),
      // Stmt::Expr(expr_stmt) => self.walk_expr_stmt(expr_stmt),
      _ => {}
    }
  }

  fn walk_decl(&mut self, decl: Decl) {
    match decl {
      // Decl::Class(class_decl) => self.walk_class_decl(class_decl),
      Decl::Fn(fn_decl) => {
        let current_scope = self.get_current_scope();
        let name = fn_decl.ident.sym.to_string();
        let fn_binding = Binding {
          kind: BindingKind::Function,
          name,
        };

        let function_node = Box::new(AstNode::FnDecl(fn_decl.clone()));
        let function_scope = Rc::new(RefCell::new(Scope::new(
          function_node.clone(),
          ScopeKind::Function,
        )));
        {
          let mut mut_scope = current_scope.borrow_mut();
          mut_scope.add_binding(fn_binding);
          mut_scope.add_child_scope(*function_node, function_scope.clone());
        }

        // Entering function scope
        self.scope_stack.push(function_scope);

        {
          let current_scope = self.get_current_scope();
          let mut mut_scope = current_scope.borrow_mut();

          for param in &fn_decl.function.params {
            let name = match param {
              Pat::Ident(ident) => ident.sym.to_string(),
              _ => todo!(),
            };
            let param_binding = Binding {
              kind: BindingKind::Param,
              name,
            };
            mut_scope.add_binding(param_binding);
          }
        }

        // Exiting function scope
        self.scope_stack.pop();
      }
      Decl::Var(var_decl) => {
        use swc_ecma_ast::VarDeclKind;

        let current_scope = self.get_current_scope();
        let mut mut_scope = current_scope.borrow_mut();

        let var_kind = match &var_decl.kind {
          VarDeclKind::Var => BindingKind::Var,
          VarDeclKind::Let => BindingKind::Let,
          VarDeclKind::Const => BindingKind::Const,
        };

        for decl in var_decl.decls {
          let name = match decl.name {
            Pat::Ident(ident) => ident.sym.to_string(),
            _ => todo!(),
          };

          mut_scope.add_binding(Binding {
            kind: var_kind.clone(),
            name,
          })
        }
      }
      // Decl::TsInterface(ts_interface_decl) => {
      //   self.walk_ts_interface_decl(ts_interface_decl)
      // }
      // Decl::TsTypeAlias(ts_type_alias_decl) => {
      //   self.walk_ts_type_alias_decl(ts_type_alias_decl)
      // }
      // Decl::TsEnum(ts_enum_decl) => self.walk_ts_enum_decl(ts_enum_decl),
      // Decl::TsModule(ts_module_decl) => {
      //   self.walk_ts_module_decl(ts_module_decl)
      // }
      _ => {}
    }
  }

  pub fn get_current_scope(&self) -> Rc<RefCell<Scope>> {
    if self.scope_stack.is_empty() {
      self.get_root_scope()
    } else {
      let index = self.scope_stack.len() - 1;
      let last = &self.scope_stack[index];
      last.clone()
    }
  }

  pub fn get_root_scope(&self) -> Rc<RefCell<Scope>> {
    self.root_scope.clone()
  }
}

pub trait LintTransform {
  fn enter(&self, context: &LintContext, node: AstNode) {}

  fn exit(&self, context: &LintContext, node: AstNode) {}
}
#[derive(Clone, Debug)]
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

#[derive(Clone)]
pub struct Scope {
  pub node: Box<AstNode>,
  pub kind: ScopeKind,
  pub bindings: HashMap<String, Binding>,
  pub child_scopes: HashMap<AstNode, Rc<RefCell<Scope>>>,
  // pub parent_scope:
}

impl fmt::Debug for Scope {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Scope")
      .field("kind", &self.kind)
      .field("bindings", &self.bindings)
      .field("child_scopes", &self.child_scopes.values())
      .finish()
  }
}

impl Scope {
  pub fn new(node: Box<AstNode>, kind: ScopeKind) -> Self {
    Self {
      node,
      kind,
      bindings: HashMap::new(),
      child_scopes: HashMap::new(),
    }
  }

  pub fn add_child_scope(&mut self, node: AstNode, scope: Rc<RefCell<Scope>>) {
    self.child_scopes.insert(node, scope);
  }

  pub fn get_binding() {
    todo!()
  }

  pub fn add_binding(&mut self, binding: Binding) {
    self.bindings.insert(binding.name.to_string(), binding);
  }
}
