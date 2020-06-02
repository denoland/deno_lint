// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Regex;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyCharacterClass;

impl LintRule for NoEmptyCharacterClass {
  fn new() -> Box<Self> {
    Box::new(NoEmptyCharacterClass)
  }

  fn code(&self) -> &'static str {
    "noEmptyCharacterClass"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEmptyCharacterClassVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoEmptyCharacterClassVisitor {
  context: Context,
}

impl NoEmptyCharacterClassVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyCharacterClassVisitor {
  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {
    let regex_literal = format!("/{}/", regex.exp);
    let rule_regex =
      regex::Regex::new(r"^/([^\[]|\.|\[([^\\\]]|\\.)+\])*/[gimuys]*$")
        .unwrap();
    if !rule_regex.is_match(&regex_literal) {
      self.context.add_diagnostic(
        regex.span,
        "noEmptyCharacterClass",
        "empty character class in RegExp is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_character_class_match() {
    assert_lint_err::<NoEmptyCharacterClass>(r#"/^abc[]/.test("abcdefg");"#, 0);
  }
  #[test]
  fn no_empty_character_class_test() {
    assert_lint_err::<NoEmptyCharacterClass>(
      r#""abcdefg".match(/^abc[]/);"#,
      16,
    );
  }

  #[test]
  fn empty_character_class_string() {
    assert_lint_ok::<NoEmptyCharacterClass>(r#"new RegExp("^abc[]");"#);
  }
}
