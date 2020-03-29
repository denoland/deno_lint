// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::VarDecl;
use swc_ecma_ast::VarDeclKind;

pub struct NoVar {
  context: Context,
}

impl NoVar {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoVar {
  fn walk_var_decl(&self, var_decl: VarDecl) {
    if var_decl.kind == VarDeclKind::Var {
      self.context.add_diagnostic(
        &var_decl.span,
        "noVar",
        "`var` keyword is not allowed",
      );
    }
  }
}
