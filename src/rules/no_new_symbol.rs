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
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn test_new_symbol() {
    test_lint(
      "no_new_symbol",
      "new Symbol()",
      vec![NoNewSymbol::new()],
      json!([{
        "code": "noNewSymbol",
        "message": "`Symbol` cannot be called as a constructor.",
        "location": {
          "filename": "no_new_symbol",
          "line": 1,
          "col": 0
        }
      }]),
    )
  }

  #[test]
  fn test_new_normal_class() {
    test_lint(
      "no_new_symbol",
      "new Class()",
      vec![NoNewSymbol::new()],
      json!([]),
    )
  }

  #[test]
  fn test_create_symbol() {
    test_lint(
      "no_new_symbol",
      "Symbol()",
      vec![NoNewSymbol::new()],
      json!([]),
    )
  }
}
