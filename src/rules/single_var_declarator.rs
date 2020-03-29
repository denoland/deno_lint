// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::VarDecl;

pub struct SingleVarDeclarator {
  context: Context,
}

impl SingleVarDeclarator {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for SingleVarDeclarator {
  fn walk_var_decl(&self, var_decl: VarDecl) {
    if var_decl.decls.len() > 1 {
      self.context.add_diagnostic(
        &var_decl.span,
        "singleVarDeclarator",
        "Multiple variable declarators are not allowed",
      );
    }
  }
}
