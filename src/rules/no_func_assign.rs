// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::ast_node::AstNode;
use crate::scopes::BindingKind;
use crate::scopes::LintContext;
use crate::scopes::LintTransform;
use swc_ecma_ast::Pat;
use swc_ecma_ast::PatOrExpr;

pub struct NoFuncAssign {
  context: Context,
}

impl NoFuncAssign {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl LintTransform for NoFuncAssign {
  fn enter(&self, context: &LintContext, node: AstNode) {
    if let AstNode::AssignExpr(assign_expr) = node {
      let scope = context.get_current_scope();

      let ident = match &assign_expr.left {
        PatOrExpr::Expr(_) => return,
        PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
          Pat::Ident(ident) => ident.sym.to_string(),
          _ => return,
        },
      };

      let scope_bor = scope.borrow();
      if let Some(binding) = scope_bor.get_binding(&ident) {
        if binding.kind == BindingKind::Function {
          self.context.add_diagnostic(
            assign_expr.span,
            "noFuncAssign",
            "Reassigning function declaration is not allowed",
          );
        }
      }
    }
  }
}
