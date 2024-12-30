// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Expr, UnaryExpr, UnaryOp};
use deno_ast::SourceRanged;
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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoDeleteVarHandler.traverse(program, context);
  }
}

struct NoDeleteVarHandler;

impl Handler for NoDeleteVarHandler {
  fn unary_expr(&mut self, unary_expr: &UnaryExpr, ctx: &mut Context) {
    if unary_expr.op() != UnaryOp::Delete {
      return;
    }

    if let Expr::Ident(_) = unary_expr.arg {
      ctx.add_diagnostic_with_hint(
        unary_expr.range(),
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
