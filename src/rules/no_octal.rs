// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use once_cell::sync::Lazy;
use regex::Regex;
use swc_ecmascript::ast::Number;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoOctal;

impl LintRule for NoOctal {
  fn new() -> Box<Self> {
    Box::new(NoOctal)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-octal"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoOctalVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoOctalVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoOctalVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoOctalVisitor<'c> {
  fn visit_number(&mut self, literal_num: &Number, _parent: &dyn Node) {
    static OCTAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^0[0-9]").unwrap());

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
  fn no_octal_valid() {
    assert_lint_ok! {
      NoOctal,
      "7",
      "\"07\"",
      "0x08",
      "-0.01",
    };
  }

  #[test]
  fn no_octal_invalid() {
    assert_lint_err::<NoOctal>("07", 0);
    assert_lint_err::<NoOctal>("let x = 7 + 07", 12);
  }
}
