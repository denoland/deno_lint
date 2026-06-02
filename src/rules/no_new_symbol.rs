// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Expression, NewExpression, Program};

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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoNewSymbolHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoNewSymbolHandler;

impl Handler<'_> for NoNewSymbolHandler {
  fn new_expression(&mut self, new_expr: &NewExpression, ctx: &mut Context) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      if ident.name.as_str() == "Symbol"
        && ctx.scope().var_by_name(ident.name.as_str()).is_none()
      {
        ctx.add_diagnostic(new_expr.span, CODE, MESSAGE);
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
