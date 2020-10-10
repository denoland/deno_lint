// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::VarDecl;
use swc_ecmascript::ast::VarDeclKind;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoVar;

impl LintRule for NoVar {
  fn new() -> Box<Self> {
    Box::new(NoVar)
  }

  fn code(&self) -> &'static str {
    "no-var"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoVarVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoVarVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoVarVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoVarVisitor<'c> {
  noop_visit_type!();

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.kind == VarDeclKind::Var {
      self.context.add_diagnostic(
        var_decl.span,
        "no-var",
        "`var` keyword is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_var_test() {
    assert_lint_err::<NoVar>(
      r#"var someVar = "someString"; const c = "c"; let a = "a";"#,
      0,
    );
  }
}
