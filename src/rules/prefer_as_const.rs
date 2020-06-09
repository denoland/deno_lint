// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsAsExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct PreferAsConst;

impl LintRule for PreferAsConst {
  fn new() -> Box<Self> {
    Box::new(PreferAsConst)
  }

  fn code(&self) -> &'static str {
    "no-var"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = PreferAsConstVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct PreferAsConstVisitor {
  context: Context,
}

impl PreferAsConstVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for PreferAsConstVisitor {
  fn visit_ts_as_expr(&mut self, as_expr: &TsAsExpr, _parent: &dyn Node) {
   println!("{:?}",as_expr);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_var_test() {
    assert_lint_err::<PreferAsConst>(
      r#"var someVar = "someString"; const c = "c"; let a = "a";"#,
      0,
    );
  }
}
