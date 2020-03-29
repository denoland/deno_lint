// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::TsInterfaceDecl;

pub struct NoEmptyInterface {
  context: Context,
}

impl NoEmptyInterface {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoEmptyInterface {
  fn walk_ts_interface_decl(&self, interface_decl: TsInterfaceDecl) {
    if interface_decl.body.body.is_empty() {
      self.context.add_diagnostic(
        &interface_decl.span,
        "noEmptyInterface",
        "Empty interfaces are not allowed",
      );
    }
  }
}
