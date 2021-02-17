// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};

pub struct NoWith;

const CODE: &str = "no-with";
const MESSAGE: &str = "`with` statement is not allowed";

impl LintRule for NoWith {
  fn new() -> Box<Self> {
    Box::new(NoWith)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    _context: &mut Context,
    _program: ProgramRef<'_>,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    NoWithHandler.traverse(program, context);
  }
}

struct NoWithHandler;

impl Handler for NoWithHandler {
  fn with_stmt(
    &self,
    with_stmt: &dprint_swc_ecma_ast_view::WithStmt,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic(with_stmt.inner.span, CODE, MESSAGE);
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
