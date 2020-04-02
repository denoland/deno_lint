// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use std::collections::HashSet;
use swc_common::Spanned;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDuplicateCase {
  context: Context,
}

impl NoDuplicateCase {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDuplicateCase {
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

        if seen.get(&test_txt).is_some() {
          self.context.add_diagnostic(
            span,
            "noDuplicateCase",
            "Duplicate values in `case` are not allowed",
          );
        } else {
          seen.insert(test_txt);
        }
      }
    }
  }
}
