// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::with_ast_view;

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
    _program: &swc_ecmascript::ast::Program,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context,
    program_info: dprint_swc_ecma_ast_view::ProgramInfo<'a>,
  ) {
    with_ast_view(program_info, |pg| {
      NoWithHandler.traverse(pg, context);
    });
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
