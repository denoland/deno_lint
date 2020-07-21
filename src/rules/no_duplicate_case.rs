// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use std::collections::HashSet;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct NoDuplicateCase;

impl LintRule for NoDuplicateCase {
  fn new() -> Box<Self> {
    Box::new(NoDuplicateCase)
  }

  fn code(&self) -> &'static str {
    "no-duplicate-case"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoDuplicateCaseVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDuplicateCaseVisitor {
  context: Arc<Context>,
}

impl NoDuplicateCaseVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoDuplicateCaseVisitor {
  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecma_ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    // Works like in ESLint - by comparing text repr of case statement
    let mut seen: HashSet<String> = HashSet::new();

    for case in &switch_stmt.cases {
      if let Some(test) = &case.test {
        let span = test.span();
        let test_txt = self.context.source_map.span_to_snippet(span).unwrap();
        if !seen.insert(test_txt) {
          self.context.add_diagnostic(
            span,
            "no-duplicate-case",
            "Duplicate values in `case` are not allowed",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_duplicate_case_test() {
    assert_lint_err_on_line::<NoDuplicateCase>(
      r#"
const someText = "some text";
switch (someText) {
    case "a":
        break;
    case "b":
        break;
    case "a":
        break;
    default:
        break;
}
      "#,
      8,
      9,
    );
  }
}
