// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Expr, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoNewSymbol;

impl LintRule for NoNewSymbol {
  fn new() -> Box<Self> {
    Box::new(NoNewSymbol)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }
  fn code(&self) -> &'static str {
    "no-new-symbol"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoNewSymbolVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoNewSymbolVisitor {
  context: Arc<Context>,
}

impl NoNewSymbolVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoNewSymbolVisitor {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if ident.sym == *"Symbol" {
        self.context.add_diagnostic(
          new_expr.span,
          "no-new-symbol",
          "`Symbol` cannot be called as a constructor.",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn test_new_symbol() {
    assert_lint_err::<NoNewSymbol>("new Symbol()", 0);
  }

  #[test]
  fn test_new_normal_class() {
    assert_lint_ok::<NoNewSymbol>("new Class()");
  }

  #[test]
  fn test_create_symbol() {
    assert_lint_ok::<NoNewSymbol>("Symbol()");
  }
}
