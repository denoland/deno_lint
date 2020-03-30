// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::FnDecl;

pub struct NoEmptyFunction {
  context: Context,
}

impl NoEmptyFunction {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoEmptyFunction {
  fn walk_fn_decl(&self, fn_decl: FnDecl) {
    if fn_decl.function.body.unwrap().stmts.is_empty() {
      self.context.add_diagnostic(
        &fn_decl.function.span,
        "noEmptyFunction",
        "Empty functions are not allowed",
      )
    }
  }
}
