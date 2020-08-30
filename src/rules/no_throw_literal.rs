// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Expr, ThrowStmt};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoThrowLiteral;

impl LintRule for NoThrowLiteral {
  fn new() -> Box<Self> {
    Box::new(NoThrowLiteral)
  }

  fn code(&self) -> &'static str {
    "no-throw-literal"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoThrowLiteralVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoThrowLiteralVisitor {
  context: Arc<Context>,
}

impl NoThrowLiteralVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoThrowLiteralVisitor {
  noop_visit_type!();

  fn visit_throw_stmt(&mut self, throw_stmt: &ThrowStmt, _parent: &dyn Node) {
    match &*throw_stmt.arg {
      Expr::Lit(_) => self.context.add_diagnostic(
        throw_stmt.span,
        "no-throw-literal",
        "expected an error object to be thrown",
      ),
      Expr::Ident(ident) if ident.sym == *"undefined" => {
        self.context.add_diagnostic(
          throw_stmt.span,
          "no-throw-literal",
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
  use crate::test_util::*;

  #[test]
  fn check_literal() {
    assert_lint_err::<NoThrowLiteral>("throw 'kumiko'", 0);
    assert_lint_err::<NoThrowLiteral>("throw true", 0);
    assert_lint_err::<NoThrowLiteral>("throw 1096", 0);
    assert_lint_err::<NoThrowLiteral>("throw null", 0);
  }

  #[test]
  fn check_undefined() {
    assert_lint_err::<NoThrowLiteral>("throw undefined", 0);
  }

  #[test]
  fn check_variable() {
    assert_lint_ok::<NoThrowLiteral>("throw e");
  }
}
