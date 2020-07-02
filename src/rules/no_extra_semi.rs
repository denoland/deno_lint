#![allow(unused, dead_code)]
// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{EmptyStmt, VarDecl};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoExtraSemi;

impl LintRule for NoExtraSemi {
  fn new() -> Box<Self> {
    Box::new(NoExtraSemi)
  }

  fn code(&self) -> &'static str {
    "no-extra-semi"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoExtraSemiVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoExtraSemiVisitor {
  context: Context,
}

impl NoExtraSemiVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoExtraSemiVisitor {
  fn visit_module(
    &mut self,
    module: &swc_ecma_ast::Module,
    _parent: &dyn Node,
  ) {
    // TODO(magurotuna)
    dbg!(module);
  }

  fn visit_empty_stmt(&mut self, empty_stmt: &EmptyStmt, _parent: &dyn Node) {
    // TODO(magurotuna)
    dbg!(empty_stmt);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn hogepiyo() {
    // TODO(magurotuna)
    assert_lint_err::<NoExtraSemi>("class A { ; }", 0);
    panic!();
  }

  #[test]
  fn no_extra_semi_valid() {
    assert_lint_ok::<NoExtraSemi>("var x = 5;");
    assert_lint_ok::<NoExtraSemi>("function foo(){}");
    assert_lint_ok::<NoExtraSemi>("for(;;);");
    assert_lint_ok::<NoExtraSemi>("while(0);");
    assert_lint_ok::<NoExtraSemi>("do;while(0);");
    assert_lint_ok::<NoExtraSemi>("for(a in b);");
    assert_lint_ok::<NoExtraSemi>("for(a of b);");
    assert_lint_ok::<NoExtraSemi>("if(true);");
    assert_lint_ok::<NoExtraSemi>("if(true); else;");
    assert_lint_ok::<NoExtraSemi>("foo: ;");
    assert_lint_ok::<NoExtraSemi>("with(foo);");
    assert_lint_ok::<NoExtraSemi>("class A { }");
    assert_lint_ok::<NoExtraSemi>("var A = class { };");
    assert_lint_ok::<NoExtraSemi>("class A { a() { this; } }");
    assert_lint_ok::<NoExtraSemi>("var A = class { a() { this; } };");
    assert_lint_ok::<NoExtraSemi>("class A { } a;");
  }

  #[test]
  fn no_extra_semi_invalid() {
    assert_lint_err::<NoExtraSemi>("var x = 5;;", 0);
    assert_lint_err::<NoExtraSemi>("function foo(){};", 0);
    assert_lint_err::<NoExtraSemi>("for(;;);;", 0);
    assert_lint_err::<NoExtraSemi>("while(0);;", 0);
    assert_lint_err::<NoExtraSemi>("do;while(0);;", 0);
    assert_lint_err::<NoExtraSemi>("for(a in b);;", 0);
    assert_lint_err::<NoExtraSemi>("for(a of b);;", 0);
    assert_lint_err::<NoExtraSemi>("if(true);;", 0);
    assert_lint_err::<NoExtraSemi>("if(true){} else;;", 0);
    assert_lint_err::<NoExtraSemi>("if(true){;} else {;}", 0);
    assert_lint_err::<NoExtraSemi>("foo:;;", 0);
    assert_lint_err::<NoExtraSemi>("with(foo);;", 0);
    assert_lint_err::<NoExtraSemi>("with(foo){;}", 0);
    assert_lint_err::<NoExtraSemi>("class A { ; }", 0);
    assert_lint_err::<NoExtraSemi>("class A { /*a*/; }", 0);
    assert_lint_err::<NoExtraSemi>("class A { ; a() {} }", 0);
    assert_lint_err::<NoExtraSemi>("class A { a() {}; }", 0);
    assert_lint_err::<NoExtraSemi>("class A { a() {}; b() {} }", 0);
    assert_lint_err::<NoExtraSemi>("class A {; a() {}; b() {}; }", 0);
    assert_lint_err::<NoExtraSemi>("class A { a() {}; get b() {} }", 0);
  }
}
