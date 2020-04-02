// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoSparseArray {
  context: Context,
}

impl NoSparseArray {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoSparseArray {
  fn visit_array_lit(
    &mut self,
    array_lit: &swc_ecma_ast::ArrayLit,
    _parent: &dyn Node,
  ) {
    if array_lit.elems.iter().any(|e| e.is_none()) {
      self.context.add_diagnostic(
        array_lit.span,
        "noSparseArray",
        "Sparse arrays are not allowed",
      );
    }
  }
}
