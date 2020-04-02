// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::VarDecl;
use swc_ecma_ast::VarDeclKind;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoVar {
  context: Context,
}

impl NoVar {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoVar {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.kind == VarDeclKind::Var {
      self.context.add_diagnostic(
        &var_decl.span,
        "noVar",
        "`var` keyword is not allowed",
      );
    }
  }
}
