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

#[derive(Clone, Debug)]
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
  pub span: Span,
  pub child_scopes: Vec<Scope>,
}

struct ScopeVisitor {

}

impl Visit for ScopeVisitor {
  fn visit_module(&mut self, module: &swc_ecma_ast::Module, parent: &dyn Node) {
    let mut program_scope = Scope {
      kind: ScopeKind::Program,
      span: module.span,
      child_scopes: vec![],
    };

    let module_scope = Scope {
      kind: ScopeKind::Module,
      span: module.span,
      child_scopes: vec![],
    };
    program_scope.child_scopes.push(module_scope);

    swc_ecma_visit::visit_module(self, module, parent);
  }
}



#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scopes() {
    let scope_visitor = ScopeVisitor {

    };

    scope_visitor.visit_module(module, module);
  }
}