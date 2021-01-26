// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::{handle_node, Handler};
use dprint_swc_ecma_ast_view::{with_ast_view, SourceFileInfo};
use swc_ecmascript::ast::{Program, WithStmt};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

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

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = NoWithVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: &Program,
  ) {
    if let Program::Module(module) = program {
      let info = SourceFileInfo {
        module,
        source_file: None,
        tokens: None,
        comments: None,
      };

      with_ast_view(info, |module| {
        let mut handler = NoWithVisitor::new(context);
        handle_node(module, &mut handler);
      });
    } else {
      self.lint_program(context, program);
    }
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

impl<'c> Visit for NoWithVisitor<'c> {
  noop_visit_type!();

  fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _parent: &dyn Node) {
    self.context.add_diagnostic(with_stmt.span, CODE, MESSAGE);
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

      // This is the same as the above one in the sense that what is tested,
      // but it's present only for with_ast_view testing, which requires that the code is parsed as
      // Module, not as Script.
      "with (someVar) { console.log('asdf'); } export const foo = 42;": [{ col: 0, message: MESSAGE }],
    }
  }
}
