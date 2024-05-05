// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{NodeTrait, VarDecl, VarDeclKind};
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct NoVar;

const MESSAGE: &str = "`var` keyword is not allowed.";
const CODE: &str = "no-var";

impl LintRule for NoVar {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoVarHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_var.md")
  }
}

struct NoVarHandler;

impl Handler for NoVarHandler {
  fn var_decl(&mut self, var_decl: &VarDecl, ctx: &mut Context) {
    if var_decl.decl_kind() == VarDeclKind::Var {
      let range = var_decl.tokens().first().unwrap().range();
      ctx.add_diagnostic(range, CODE, MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_var_valid() {
    assert_lint_ok!(NoVar, r#"let foo = 0; const bar = 1"#,);
  }

  #[test]
  fn no_var_invalid() {
    assert_lint_err!(
      NoVar,
      "var foo = 0;": [{
        col: 0,
        message: MESSAGE,
      }],
      "let foo = 0; var bar = 1;": [{
        col: 13,
        message: MESSAGE,
      }],
      "let foo = 0; var bar = 1; var x = 2;": [
        {
          col: 13,
          message: MESSAGE,
        },
        {
          col: 26,
          message: MESSAGE,
        }
      ]
    );
  }
}
