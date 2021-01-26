// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Expr, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoNewSymbol;

const CODE: &str = "no-new-symbol";
const MESSAGE: &str = "`Symbol` cannot be called as a constructor.";

impl LintRule for NoNewSymbol {
  fn new() -> Box<Self> {
    Box::new(NoNewSymbol)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoNewSymbolVisitor::new(context);
    program.visit_all_with(program, &mut visitor);
  }
}

struct NoNewSymbolVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoNewSymbolVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> VisitAll for NoNewSymbolVisitor<'c> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if ident.sym == *"Symbol" {
        self.context.add_diagnostic(new_expr.span, CODE, MESSAGE);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_new_symbol_valid() {
    assert_lint_ok! {
      NoNewSymbol,
      "new Class()",
      "Symbol()",
    };
  }

  #[test]
  fn no_new_symbol_invalid() {
    assert_lint_err! {
      NoNewSymbol,
      "new Symbol()": [{ col: 0, message: MESSAGE }],
      // nested
      "new class { foo() { new Symbol(); } }": [{ col: 20, message: MESSAGE }],
    };
  }
}
