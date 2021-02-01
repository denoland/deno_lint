// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{with_ast_view, ProgramInfo};

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
    unimplemented!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let info = ProgramInfo {
      program,
      source_file: None,
      tokens: None,
      comments: None,
    };

    with_ast_view(info, |module| {
      let mut handler = NoWithVisitor::new(context);
      handler.traverse(module);
    });
  }
}

struct NoWithVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoWithVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Handler for NoWithVisitor<'c> {
  fn with_stmt(&mut self, with_stmt: &dprint_swc_ecma_ast_view::WithStmt) {
    self
      .context
      .add_diagnostic(with_stmt.inner.span, CODE, MESSAGE);
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
