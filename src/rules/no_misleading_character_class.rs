// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::sync::Arc;
use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Lit;
use swc_ecmascript::ast::Module;
use swc_ecmascript::visit::{Node, Visit};

pub struct NoMisleadingCharacterClass;

impl LintRule for NoMisleadingCharacterClass {
  fn new() -> Box<Self> {
    Box::new(NoMisleadingCharacterClass)
  }

  fn code(&self) -> &'static str {
    "no-misleading-character-class"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoMisleadingCharacterClassVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoMisleadingCharacterClassVisitor {
  context: Arc<Context>,
}

impl NoMisleadingCharacterClassVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-misleading-character-class",
      "Characters which are made of multiple code points are not allowed in a character class syntax",
    );
  }

  // Helper functions will go here
}

impl Visit for NoMisleadingCharacterClassVisitor {
  fn visit_program(&mut self, program: &Program, _parent: &dyn Node) {}

  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn passing() {}

  // TODO(humancalico) make these tests pass
  #[test]
  #[ignore]
  fn failing() {}
}
