// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::Module;
use swc_ecma_visit::{Node, Visit};

pub struct NoConstantCondition;

impl LintRule for NoConstantCondition {
  fn new() -> Box<Self> {
    Box::new(NoConstantCondition)
  }

  fn code(&self) -> &'static str {
    "no-constant-condition"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoConstantCondition::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoConstantConditionVisitor {
  context: Context,
}

impl NoConstantConditionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-constant-condition",
      "Use of a constant expressions as conditions is not allowed.",
    );
  }

  fn check_condition(&self, condition: &Expr) {
    if let Expr::Lit(lit_node) = &condition {
      self.add_diagnostic(lit_node.span())
    }
    if let Expr::Arrow(arrow) = &condition {
      self.add_diagnostic(arrow.span)
    }
  }
}

impl Visit for NoConstantConditionVisitor {
  fn visit_if_stmt(
    &mut self,
    if_stmt: &swc_ecma_ast::IfStmt,
    _parent: &dyn Node,
  ) {
    self.check_condition(&if_stmt.test);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_constant_condition_lit() {
    assert_lint_err::<NoConstantCondition>(
      r#"if ("some str") { console.log("Constant Condition encountered")} "#,
      13,
    );
  }
}
