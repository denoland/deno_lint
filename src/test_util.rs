// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use crate::rules::LintRule;
use crate::Linter;
use serde_json::Value;

pub fn test_lint(
  filename: &str,
  source_code: &str,
  rules: Vec<Box<dyn LintRule>>,
  expected_diagnostics: Value,
) {
  let mut linter = Linter::default();
  let diagnostics = linter
    .lint(filename.to_string(), source_code.to_string(), rules)
    .expect("Failed to lint");
  let serialized_diagnostics = serde_json::to_value(diagnostics).unwrap();
  assert_eq!(serialized_diagnostics, expected_diagnostics);
}
