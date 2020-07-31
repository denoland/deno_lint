// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::VarDecl;
use crate::swc_ecma_ast::VarDeclKind;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoVar;

impl LintRule for NoVar {
  fn new() -> Box<Self> {
    Box::new(NoVar)
  }

  fn code(&self) -> &'static str {
    "no-var"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoVarVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoVarVisitor {
  context: Arc<Context>,
}

impl NoVarVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoVarVisitor {
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
