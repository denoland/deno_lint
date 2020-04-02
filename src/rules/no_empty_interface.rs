// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::TsInterfaceDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyInterface {
  context: Context,
}

impl NoEmptyInterface {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyInterface {
  fn visit_ts_interface_decl(
    &mut self,
    interface_decl: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    if interface_decl.body.body.is_empty() {
      self.context.add_diagnostic(
        interface_decl.span,
        "noEmptyInterface",
        "Empty interfaces are not allowed",
      );
    }
  }
}
