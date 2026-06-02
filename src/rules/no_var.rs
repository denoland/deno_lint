// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Program, VariableDeclaration, VariableDeclarationKind,
};
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoVar;

const MESSAGE: &str = "`var` keyword is not allowed.";
const CODE: &str = "no-var";

impl LintRule for NoVar {
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
    let mut handler = NoVarHandler {
      in_ts_module_block: 0,
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoVarHandler {
  in_ts_module_block: u32,
}

impl Handler<'_> for NoVarHandler {
  fn ts_module_block(
    &mut self,
    _n: &deno_ast::oxc::ast::ast::TSModuleBlock,
    _ctx: &mut Context,
  ) {
    self.in_ts_module_block += 1;
  }

  fn variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration,
    ctx: &mut Context,
  ) {
    if self.in_ts_module_block > 0 {
      return;
    }

    if var_decl.kind == VariableDeclarationKind::Var {
      // Span for just the "var" keyword (3 chars)
      let range = Span::new(var_decl.span.start, var_decl.span.start + 3);
      ctx.add_diagnostic(range, CODE, MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_var_valid() {
    assert_lint_ok!(
      NoVar,
      r#"let foo = 0; const bar = 1"#,
      r#"declare global {
  namespace globalThis {
    var test: string
  }
}"#,
      r#"declare global {
  var test: string
}"#,
    );
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
      ],
    );
  }
}
