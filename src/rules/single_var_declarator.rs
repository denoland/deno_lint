// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::VarDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct SingleVarDeclarator {
  context: Context,
}

impl SingleVarDeclarator {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for SingleVarDeclarator {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.decls.len() > 1 {
      self.context.add_diagnostic(
        var_decl.span,
        "singleVarDeclarator",
        "Multiple variable declarators are not allowed",
      );
    }
  }
}
