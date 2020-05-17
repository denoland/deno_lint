// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_atoms::js_word;
use swc_ecma_ast::{Expr, ThrowStmt};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoThrowLiteral;

impl LintRule for NoThrowLiteral {
  fn new() -> Box<Self> {
    Box::new(NoThrowLiteral)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoThrowLiteralVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoThrowLiteralVisitor {
  context: Context,
}

impl NoThrowLiteralVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoThrowLiteralVisitor {
  fn visit_throw_stmt(&mut self, throw_stmt: &ThrowStmt, _parent: &dyn Node) {
    match &*throw_stmt.arg {
      Expr::Lit(_) => self.context.add_diagnostic(
        throw_stmt.span,
        "noThrowLiteral",
        "expected an error object to be thrown",
      ),
      Expr::Ident(ident) if ident.sym == js_word!("undefined") => {
        self.context.add_diagnostic(
          throw_stmt.span,
          "noThrowLiteral",
          "do not throw undefined",
        )
      }
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn check_literal() {
    test_lint(
      "no_throw_literal",
      "throw 'kumiko'",
      vec![NoThrowLiteral::new()],
      json!([{
        "code": "noThrowLiteral",
        "message": "expected an error object to be thrown",
        "location": {
          "filename": "no_throw_literal",
          "line": 1,
          "col": 0
        }
      }]),
    );
  }

  #[test]
  fn check_undefined() {
    test_lint(
      "no_throw_literal",
      "throw undefined",
      vec![NoThrowLiteral::new()],
      json!([{
        "code": "noThrowLiteral",
        "message": "do not throw undefined",
        "location": {
          "filename": "no_throw_literal",
          "line": 1,
          "col": 0
        }
      }]),
    );
  }

  #[test]
  fn check_variable() {
    test_lint(
      "no_throw_lietral",
      "throw e",
      vec![NoThrowLiteral::new()],
      json!([]),
    );
  }
}
