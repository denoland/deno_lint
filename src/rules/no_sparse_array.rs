// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoSparseArray;

impl LintRule for NoSparseArray {
  fn new() -> Box<Self> {
    Box::new(NoSparseArray)
  }

  fn code(&self) -> &'static str {
    "no-sparse-array"
  }

  fn lint_module(&self, context: Context, module: &swc_ecma_ast::Module) {
    let mut visitor = NoSparseArrayVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoSparseArrayVisitor {
  context: Context,
}

impl NoSparseArrayVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoSparseArrayVisitor {
  fn visit_array_lit(
    &mut self,
    array_lit: &swc_ecma_ast::ArrayLit,
    _parent: &dyn Node,
  ) {
    if array_lit.elems.iter().any(|e| e.is_none()) {
      self.context.add_diagnostic(
        array_lit.span,
        "no-sparse-array",
        "Sparse arrays are not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_sparse_array_test() {
    assert_lint_ok::<NoSparseArray>("const sparseArray1 = [1,null,3];");
    assert_lint_err::<NoSparseArray>("const sparseArray = [1,,3];", 20);
  }
}
