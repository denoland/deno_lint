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
  let mut linter = Linter::default();
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

fn lint<T: LintRule + 'static>(source: &str) -> Vec<LintDiagnostic> {
  let mut linter = Linter::default();
  let rule = T::new();
  linter
    .lint(
      "deno_lint_test.tsx".to_string(),
      source.to_string(),
      vec![rule],
    )
    .expect("Failed to lint")
}

fn assert_diagnostic(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
) {
  if diagnostic.code == code
    && diagnostic.location.line == line
    && diagnostic.location.col == col
  {
    return;
  }
  panic!(format!(
    "expect diagnostics {} at {}:{} to be {} at {}:{}",
    diagnostic.code,
    diagnostic.location.line,
    diagnostic.location.col,
    code,
    line,
    col
  ))
}

pub fn assert_lint_ok<T: LintRule + 'static>(cases: Vec<&str>) {
  for source in cases {
    let diagnostics = lint::<T>(source);
    assert!(diagnostics.is_empty());
  }
}

#[allow(dead_code)]
pub fn assert_lint_err<T: LintRule + 'static>(
  source: &str,
  code: &str,
  col: usize,
) {
  assert_lint_err_on_line::<T>(source, code, 0, col)
}

pub fn assert_lint_err_on_line<T: LintRule + 'static>(
  source: &str,
  code: &str,
  line: usize,
  col: usize,
) {
  let diagnostics = lint::<T>(source);
  assert!(diagnostics.len() == 1);
  assert_diagnostic(&diagnostics[0], code, line, col);
}

#[allow(dead_code)]
pub fn assert_lint_err_n<T: LintRule + 'static>(
  source: &str,
  expected: Vec<(&str, usize)>,
) {
  let mut real: Vec<(&str, usize, usize)> = Vec::new();
  for x in expected {
    let (code, col) = x;
    real.push((code, 0, col));
  }
  assert_lint_err_on_line_n::<T>(source, real)
}

pub fn assert_lint_err_on_line_n<T: LintRule + 'static>(
  source: &str,
  expected: Vec<(&str, usize, usize)>,
) {
  let diagnostics = lint::<T>(source);
  assert!(diagnostics.len() == expected.len());
  for i in 0..diagnostics.len() {
    let (code, line, col) = expected[i];
    assert_diagnostic(&diagnostics[i], code, line, col);
  }
}
