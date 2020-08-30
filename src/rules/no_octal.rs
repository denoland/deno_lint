// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Number;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoOctal;

impl LintRule for NoOctal {
  fn new() -> Box<Self> {
    Box::new(NoOctal)
  }

  fn code(&self) -> &'static str {
    "no-octal"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoOctalVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoOctalVisitor {
  context: Arc<Context>,
}

impl NoOctalVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoOctalVisitor {
  fn visit_number(&mut self, literal_num: &Number, _parent: &dyn Node) {
    lazy_static! {
      static ref OCTAL: regex::Regex = regex::Regex::new(r"^0[0-9]").unwrap();
    }

    let raw_number = self
      .context
      .source_map
      .span_to_snippet(literal_num.span)
      .expect("error in loading snippet");

    if OCTAL.is_match(&raw_number) {
      self.context.add_diagnostic(
        literal_num.span,
        "no-octal",
        "`Octal number` is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn test_literal_octal() {
    assert_lint_err::<NoOctal>("07", 0);
  }

  #[test]
  fn test_operand_octal() {
    assert_lint_err::<NoOctal>("let x = 7 + 07", 12);
  }

  #[test]
  fn test_octals_valid() {
    assert_lint_ok_n::<NoOctal>(vec!["7", "\"07\"", "0x08", "-0.01"]);
  }
}
