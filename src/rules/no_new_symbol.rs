// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_atoms::js_word;
use swc_ecma_ast::{Expr, NewExpr};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoNewSymbol;

impl LintRule for NoNewSymbol {
  fn new() -> Box<Self> {
    Box::new(NoNewSymbol)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoNewSymbolVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoNewSymbolVisitor {
  context: Context,
}

impl NoNewSymbolVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoNewSymbolVisitor {
  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if ident.sym == js_word!("Symbol") {
        self.context.add_diagnostic(
          new_expr.span,
          "noNewSymbol",
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
    assert_lint_err::<NoNewSymbol>("new Symbol()", "noNewSymbol", 0);
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
