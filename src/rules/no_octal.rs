// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Number;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoOctal;

impl LintRule for NoOctal {
  fn new() -> Box<Self> {
    Box::new(NoOctal)
  }

  fn code(&self) -> &'static str {
    "no-octal"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoOctalVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoOctalVisitor {
  context: Context,
}

impl NoOctalVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoOctalVisitor {
  fn visit_number(&mut self, literal_num: &Number, _parent: &dyn Node) {
    lazy_static! {
      static ref OCTAL: regex::Regex = regex::Regex::new(
        r"^0[0-9]"
      )
      .unwrap();
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
  fn test_new_normal_number() {
    assert_lint_ok::<NoOctal>("7");
  }

  #[test]
  fn test_string_octal_number() {
    assert_lint_ok::<NoOctal>("\"07\"");
  }

  #[test]
  fn test_zero_number() {
    assert_lint_ok::<NoOctal>("0");
  }
}
