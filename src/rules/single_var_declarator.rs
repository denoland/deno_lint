// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{Program, VariableDeclaration};
use derive_more::Display;

#[derive(Debug)]
pub struct SingleVarDeclarator;

const CODE: &str = "single-var-declarator";

#[derive(Display)]
enum SingleVarDeclaratorMessage {
  #[display(fmt = "Multiple variable declarators are not allowed")]
  Unexpected,
}

impl LintRule for SingleVarDeclarator {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = SingleVarDeclaratorHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct SingleVarDeclaratorHandler;

impl Handler<'_> for SingleVarDeclaratorHandler {
  fn variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration,
    ctx: &mut Context,
  ) {
    if var_decl.declarations.len() > 1 {
      ctx.add_diagnostic(
        var_decl.span,
        CODE,
        SingleVarDeclaratorMessage::Unexpected,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_var_declarator_invalid() {
    assert_lint_err! {
      SingleVarDeclarator,
      r#"const a1 = "a", b1 = "b", c1 = "c";"#: [
      {
        col: 0,
        message: SingleVarDeclaratorMessage::Unexpected,
      }],
      r#"let a2 = "a", b2 = "b", c2 = "c";"#: [
      {
        col: 0,
        message: SingleVarDeclaratorMessage::Unexpected,
      }],
      r#"var a3 = "a", b3 = "b", c3 = "c";"#: [
      {
        col: 0,
        message: SingleVarDeclaratorMessage::Unexpected,
      }],
    }
  }
}
