// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoSparseArrays;

impl LintRule for NoSparseArrays {
  fn new() -> Box<Self> {
    Box::new(NoSparseArrays)
  }

  fn code(&self) -> &'static str {
    "no-sparse-arrays"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoSparseArraysVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoSparseArraysVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoSparseArraysVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoSparseArraysVisitor<'c> {
  noop_visit_type!();

  fn visit_array_lit(
    &mut self,
    array_lit: &swc_ecmascript::ast::ArrayLit,
    _parent: &dyn Node,
  ) {
    if array_lit.elems.iter().any(|e| e.is_none()) {
      self.context.add_diagnostic(
        array_lit.span,
        "no-sparse-arrays",
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
  fn no_sparse_arrays_test() {
    assert_lint_ok::<NoSparseArrays>("const sparseArray1 = [1,null,3];");
    assert_lint_err::<NoSparseArrays>("const sparseArray = [1,,3];", 20);
  }
}
