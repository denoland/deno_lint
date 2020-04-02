// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Expr;
use swc_ecma_ast::UnaryExpr;
use swc_ecma_ast::UnaryOp;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDeleteVar;

impl LintRule for NoDeleteVar {
  fn new() -> Box<Self> {
    Box::new(NoDeleteVar)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoDeleteVarVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoDeleteVarVisitor {
  context: Context,
}

impl NoDeleteVarVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDeleteVarVisitor {
  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _parent: &dyn Node) {
    if unary_expr.op != UnaryOp::Delete {
      return;
    }

    if let Expr::Ident(_) = *unary_expr.arg {
      self.context.add_diagnostic(
        unary_expr.span,
        "noDeleteVar",
        "Variables shouldn't be deleted",
      );
    }
  }
}
