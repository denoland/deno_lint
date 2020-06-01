// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use crate::linter::LintDiagnostic;
use crate::rules::LintRule;
use crate::Linter;

use serde_json::{json, Value};

pub fn test_lint(
  filename: &str,
  source_code: &str,
  rules: Vec<Box<dyn LintRule>>,
  expected_diagnostics: Value,
) {
  let mut linter: Linter = Default::default();
  let diagnostics = linter
    .lint(filename.to_string(), source_code.to_string(), rules)
    .expect("Failed to lint");

  // TODO(@disizali) refactor this to a better approach.
  // it's a temporary solution for ignoring line_src field.
  let serialized_diagnostics = if !diagnostics.is_empty() {
    let mut ignored_line_src_diagnostics = vec![];
    for diagnostic in &diagnostics {
      let LintDiagnostic {
        code,
        message,
        location,
        ..
      } = diagnostic;
      ignored_line_src_diagnostics
        .push(json!({"code":code,"message":message,"location":location}))
    }
    serde_json::to_value(ignored_line_src_diagnostics).unwrap()
  } else {
    serde_json::to_value(diagnostics).unwrap()
  };

  assert_eq!(serialized_diagnostics, expected_diagnostics);
}
