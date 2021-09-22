// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{Expr, NewExpr};
use deno_ast::swc::utils::ident::IdentLike;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::{VisitAll, VisitAllWith};
use if_chain::if_chain;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoNewSymbol;

const CODE: &str = "no-new-symbol";
const MESSAGE: &str = "`Symbol` cannot be called as a constructor.";

impl LintRule for NoNewSymbol {
  fn new() -> Arc<Self> {
    Arc::new(NoNewSymbol)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoNewSymbolVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_new_symbol.md")
  }
}

struct NoNewSymbolVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoNewSymbolVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoNewSymbolVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if_chain! {
      if let Expr::Ident(ident) = &*new_expr.callee;
      if ident.sym == *"Symbol";
      if self.context.scope().var(&ident.to_id()).is_none();
      then {
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
      // not a built-in Symbol
      r#"
function f(Symbol: typeof SomeClass) {
  const foo = new Symbol();
}
      "#,
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
