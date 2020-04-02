// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::FnDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyFunction {
  context: Context,
}

impl NoEmptyFunction {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyFunction {
  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _parent: &dyn Node) {
    let body = fn_decl.function.body.as_ref();
    if body.is_none() || body.unwrap().stmts.is_empty() {
      self.context.add_diagnostic(
        fn_decl.function.span,
        "noEmptyFunction",
        "Empty functions are not allowed",
      )
    }
  }
}
