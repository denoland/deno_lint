// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::Stmt;
use swc_ecma_ast::BlockStmt;
use swc_ecma_ast::SetterProp;
use swc_ecma_ast::Class;
use swc_ecma_ast::ClassMember;
use swc_ecma_ast::MethodKind;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoSetterReturn {
  context: Context,
}

impl NoSetterReturn {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_block_stmt(&self, block_stmt: &BlockStmt) {
    if block_stmt.stmts.iter().any(|s| match s {
      Stmt::Return(_) => true,
      _ => false,
    }) {
      self.context.add_diagnostic(
        &block_stmt.span,
        "noSetterReturn",
        "Setter return is not allowed",
      );
    }
  }
}

impl Visit for NoSetterReturn {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    for member in &class.body {
      match member {
        ClassMember::Method(class_method) => {
          if class_method.kind == MethodKind::Setter {
            if let Some(block_stmt) = &class_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        },
        ClassMember::PrivateMethod(private_method) => {
          if private_method.kind == MethodKind::Setter {
            if let Some(block_stmt) = &private_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        },
        _ => {}
      }
    }
  }

  fn visit_setter_prop(&mut self, setter_prop: &SetterProp, _parent: &dyn Node) {
    if let Some(block_stmt) = &setter_prop.body {
      self.check_block_stmt(block_stmt);
    }
  }
}
