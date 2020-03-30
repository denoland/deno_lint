// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use crate::ast_node::AstNode;
use crate::ast_node::AstNodeKind;

pub struct LintContext {
  pub root_scope: Scope,
  pub scope_stack: Vec<Scope>,
}

impl LintContext {
  pub fn new(root_scope: Scope) -> Self {
    Self {
      root_scope,
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

  pub fn get_current_scope(&self) -> &Scope {
    if self.scope_stack.is_empty() {
      &self.root_scope
    } else {
      &self.scope_stack[self.scope_stack.len() - 1]
    }
  }

  pub fn get_root_scope(&self) -> &Scope {
    &self.root_scope
  }
}

pub trait LintTransform {
  fn enter(&self, context: &LintContext, node: AstNode) {}

  fn exit(&self, context: &LintContext, node: AstNode) {}
}

pub enum ScopeKind {
  Root,
  Program,
  Function,
  Block,
  Loop,
  Class,
}

pub struct Scope {
  pub kind: ScopeKind,
  // TODO: bindings
}

impl Scope {
  pub fn get_binding() {
    todo!()
  }

  pub fn add_binding() {
    todo!()
  }
}
