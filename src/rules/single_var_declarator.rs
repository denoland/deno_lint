// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::VarDecl;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct SingleVarDeclarator;

impl LintRule for SingleVarDeclarator {
  fn new() -> Box<Self> {
    Box::new(SingleVarDeclarator)
  }

  fn code(&self) -> &'static str {
    "single-var-declarator"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = SingleVarDeclaratorVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct SingleVarDeclaratorVisitor {
  context: Arc<Context>,
}

impl SingleVarDeclaratorVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for SingleVarDeclaratorVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.decls.len() > 1 {
      self.context.add_diagnostic(
        var_decl.span,
        "single-var-declarator",
        "Multiple variable declarators are not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn single_var_declarator_test() {
    assert_lint_err::<SingleVarDeclarator>(
      r#"const a1 = "a", b1 = "b", c1 = "c";"#,
      0,
    );
    assert_lint_err::<SingleVarDeclarator>(
      r#"let a2 = "a", b2 = "b", c2 = "c";"#,
      0,
    );
    assert_lint_err::<SingleVarDeclarator>(
      r#"var a3 = "a", b3 = "b", c3 = "c";"#,
      0,
    );
  }
}
