// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Program, WithStatement};

#[derive(Debug)]
pub struct NoWith;

const CODE: &str = "no-with";
const MESSAGE: &str = "`with` statement is not allowed";

impl LintRule for NoWith {
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
    let mut handler = NoWithHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoWithHandler;

impl Handler<'_> for NoWithHandler {
  fn with_statement(&mut self, with_stmt: &WithStatement, ctx: &mut Context) {
    ctx.add_diagnostic(with_stmt.span, CODE, MESSAGE);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoWith,
      "with (someVar) { console.log('asdf'); }": [{ col: 0, message: MESSAGE }],
    }
  }
}
