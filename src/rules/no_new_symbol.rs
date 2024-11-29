// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Expr, NewExpr};
use deno_ast::SourceRanged;
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoNewSymbol;

const CODE: &str = "no-new-symbol";
const MESSAGE: &str = "`Symbol` cannot be called as a constructor.";

impl LintRule for NoNewSymbol {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoNewSymbolHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_new_symbol.md")
  }
}

struct NoNewSymbolHandler;

impl Handler for NoNewSymbolHandler {
  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    if_chain! {
      if let Expr::Ident(ident) = new_expr.callee;
      if *ident.sym() == *"Symbol";
      if ctx.scope().var(&ident.to_id()).is_none();
      then {
        ctx.add_diagnostic(new_expr.range(), CODE, MESSAGE);
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
