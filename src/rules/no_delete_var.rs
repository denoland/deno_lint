// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::UnaryExpr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoDeleteVar;

impl LintRule for NoDeleteVar {
  fn new() -> Box<Self> {
    Box::new(NoDeleteVar)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-delete-var"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoDeleteVarVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDeleteVarVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDeleteVarVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoDeleteVarVisitor<'c> {
  noop_visit_type!();

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _parent: &dyn Node) {
    if unary_expr.op != UnaryOp::Delete {
      return;
    }

    if let Expr::Ident(_) = *unary_expr.arg {
      self.context.add_diagnostic(
        unary_expr.span,
        "no-delete-var",
        "Variables shouldn't be deleted",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_delete_var_test() {
    assert_lint_err::<NoDeleteVar>(
      r#"var someVar = "someVar"; delete someVar;"#,
      25,
    );
  }
}
