// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Expression, Program, UnaryExpression, UnaryOperator};
use derive_more::Display;

#[derive(Debug)]
pub struct NoDeleteVar;

const CODE: &str = "no-delete-var";

#[derive(Display)]
enum NoDeleteVarMessage {
  #[display(fmt = "Variables shouldn't be deleted")]
  Unexpected,
}

#[derive(Display)]
enum NoDeleteVarHint {
  #[display(fmt = "Remove the deletion statement")]
  Remove,
}

impl LintRule for NoDeleteVar {
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
    let mut handler = NoDeleteVarHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoDeleteVarHandler;

impl Handler<'_> for NoDeleteVarHandler {
  fn unary_expression(
    &mut self,
    unary_expr: &UnaryExpression,
    ctx: &mut Context,
  ) {
    if unary_expr.operator != UnaryOperator::Delete {
      return;
    }

    if let Expression::Identifier(_) = &unary_expr.argument {
      ctx.add_diagnostic_with_hint(
        unary_expr.span,
        CODE,
        NoDeleteVarMessage::Unexpected,
        NoDeleteVarHint::Remove,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_delete_var_invalid() {
    assert_lint_err! {
      NoDeleteVar,
      r#"var someVar = "someVar"; delete someVar;"#: [
        {
          col: 25,
          message: NoDeleteVarMessage::Unexpected,
          hint: NoDeleteVarHint::Remove,
        }
      ],
    }
  }
}
