// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use crate::ast_node::AstNode;
use crate::ast_node::AstNodeKind;
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

pub struct LintContext {
  pub root_scope: Rc<RefCell<Scope>>,
  pub scope_stack: Vec<Rc<RefCell<Scope>>>,
  pub transforms: Vec<Box<dyn LintTransform>>,
}

impl LintContext {
  pub fn new(
    node: Box<AstNode>,
    transforms: Vec<Box<dyn LintTransform>>,
  ) -> Self {
    Self {
      root_scope: Rc::new(RefCell::new(Scope::new(
        node,
        ScopeKind::Root,
        None,
      ))),
      scope_stack: vec![],
      transforms,
    }
  }

  pub fn walk_module(&mut self, module: Module) {
    assert!(self.scope_stack.is_empty());

    let module_node = Box::new(AstNode::Module(module.clone()));
    let current_scope = self.get_current_scope();

    let module_scope = Rc::new(RefCell::new(Scope::new(
      module_node.clone(),
      ScopeKind::Module,
      Some(current_scope.clone()),
    )));
    {
      let mut s = current_scope.borrow_mut();
      s.add_child_scope(AstNode::Module(module.clone()), module_scope.clone());
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
      Stmt::Expr(expr_stmt) => self.walk_expr_stmt(expr_stmt),
      _ => {}
    }
  }

  fn walk_expr_stmt(&self, expr_stmt: ExprStmt) {
    self.walk_expression(expr_stmt.expr);
  }

  fn walk_expression(&self, expr: Box<Expr>) {
    match *expr {
      // Expr::Array(array_lit) => self.walk_array_lit(array_lit),
      // Expr::Arrow(arrow_expr) => self.walk_arrow_expr(arrow_expr),
      Expr::Assign(assign_expr) => self.walk_assign_expr(assign_expr),
      _ => {}
      // Expr::Await(await_expr) => self.walk_await_expr(await_expr),
      // Expr::Bin(bin_expr) => self.walk_bin_expr(bin_expr),
      // Expr::Call(call_expr) => self.walk_call_expr(call_expr),
      // Expr::Class(class_expr) => self.walk_class_expr(class_expr),
      // Expr::Cond(cond_expr) => self.walk_cond_expr(cond_expr),
      // Expr::Fn(fn_expr) => self.walk_fn_expr(fn_expr),
      // Expr::Ident(ident) => self.walk_identifier_reference(ident),
      // Expr::Invalid(_) => {}
      // Expr::JSXMember(jsx_member_expr) => {
      //   self.walk_jsx_member_expr(jsx_member_expr)
      // }
      // Expr::JSXNamespacedName(jsx_namespaced_name) => {
      //   self.walk_jsx_namespaced_name(jsx_namespaced_name)
      // }
      // Expr::JSXEmpty(jsx_empty_expr) => self.walk_jsx_empty(jsx_empty_expr),
      // Expr::JSXElement(jsx_element) => self.walk_jsx_element(jsx_element),
      // Expr::JSXFragment(jsx_fragment) => self.walk_jsx_fragment(jsx_fragment),
      // Expr::Member(member_expr) => self.walk_member_expr(member_expr),
      // Expr::MetaProp(meta_prop_expr) => {
      //   self.walk_meta_prop_expr(meta_prop_expr)
      // }
      // Expr::New(new_expr) => self.walk_new_expr(new_expr),
      // Expr::Lit(lit) => self.walk_lit(lit),
      // Expr::Object(object_lit) => self.walk_object_lit(object_lit),
      // Expr::OptChain(opt_chain_expr) => {
      //   self.walk_opt_chain_expr(opt_chain_expr)
      // }
      // Expr::Paren(paren_expr) => self.walk_paren_expr(paren_expr),
      // Expr::PrivateName(private_name) => self.walk_private_name(private_name),
      // Expr::Seq(seq_expr) => self.walk_seq_expr(seq_expr),
      // Expr::TaggedTpl(tagged_tpl) => self.walk_tagged_tpl(tagged_tpl),
      // Expr::This(this_expr) => self.walk_this_expr(this_expr),
      // Expr::Tpl(tpl) => self.walk_tpl(tpl),
      // Expr::TsTypeAssertion(ts_type_assertion) => {
      //   self.walk_ts_type_assertion(ts_type_assertion)
      // }
      // Expr::TsConstAssertion(ts_const_assertion) => {
      //   self.walk_ts_const_assertion(ts_const_assertion)
      // }
      // Expr::TsNonNull(ts_non_null_expr) => {
      //   self.walk_ts_non_null_expr(ts_non_null_expr)
      // }
      // Expr::TsTypeCast(ts_type_cast_expr) => {
      //   self.walk_ts_type_cast_expr(ts_type_cast_expr)
      // }
      // Expr::TsAs(ts_as_expr) => self.walk_ts_as_expr(ts_as_expr),
      // Expr::Unary(unary_expr) => self.walk_unary_expr(unary_expr),
      // Expr::Update(update_expr) => self.walk_update_expr(update_expr),
      // Expr::Yield(yield_expr) => self.walk_yield_expr(yield_expr),
    }
  }

  fn walk_assign_expr(&self, assign_expr: AssignExpr) {
    let node = AstNode::AssignExpr(assign_expr);

    for transform in &self.transforms {
      transform.enter(self, node.clone());
    }

    for transform in &self.transforms {
      transform.exit(self, node.clone());
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
          Some(current_scope.clone()),
        )));

        {
          let mut s = current_scope.borrow_mut();
          s.add_child_scope(
            AstNode::FnDecl(fn_decl.clone()),
            function_scope.clone(),
          );
          s.add_binding(fn_binding);
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

#[derive(Clone)]
pub struct Scope {
  pub node: Box<AstNode>,
  pub kind: ScopeKind,
  pub bindings: HashMap<String, Binding>,
  pub parent_scope: Option<Rc<RefCell<Scope>>>,
  pub child_scopes: HashMap<AstNode, Rc<RefCell<Scope>>>,
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
  pub fn new(
    node: Box<AstNode>,
    kind: ScopeKind,
    parent_scope: Option<Rc<RefCell<Scope>>>,
  ) -> Self {
    Self {
      node,
      kind,
      bindings: HashMap::new(),
      parent_scope,
      child_scopes: HashMap::new(),
    }
  }

  pub fn add_child_scope(&mut self, node: AstNode, scope: Rc<RefCell<Scope>>) {
    self.child_scopes.insert(node, scope);
  }

  pub fn get_binding(&self, name: &str) -> Option<Binding> {
    // TODO: lookup bindings in parent scopes
    if let Some(binding_ref) = self.bindings.get(name) {
      return Some(binding_ref.clone());
    }

    if let Some(parent_scope) = &self.parent_scope {
      let ps = parent_scope.borrow();
      return ps.get_binding(name);
    }

    None
  }

  pub fn add_binding(&mut self, binding: Binding) {
    self.bindings.insert(binding.name.to_string(), binding);
  }
}
