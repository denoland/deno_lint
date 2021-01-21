// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::VarDecl;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct SingleVarDeclarator;

impl LintRule for SingleVarDeclarator {
  fn new() -> Box<Self> {
    Box::new(SingleVarDeclarator)
  }

  fn code(&self) -> &'static str {
    "single-var-declarator"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = SingleVarDeclaratorVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct SingleVarDeclaratorVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> SingleVarDeclaratorVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for SingleVarDeclaratorVisitor<'c> {
  noop_visit_type!();

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
  fn single_var_declarator_invalid() {
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
