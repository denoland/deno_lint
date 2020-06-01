// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use crate::diagnostic::LintDiagnostic;
use crate::rules::LintRule;
use crate::Linter;

fn lint(rule: Box<dyn LintRule>, source: &str) -> Vec<LintDiagnostic> {
  let mut linter = Linter::default();
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

pub fn assert_lint_ok<T: LintRule + 'static>(source: &str) {
  let rule = T::new();
  let diagnostics = lint(rule, source);
  assert!(diagnostics.is_empty());
}

pub fn assert_lint_ok_n<T: LintRule + 'static>(cases: Vec<&str>) {
  for source in cases {
    assert_lint_ok::<T>(source);
  }
}

pub fn assert_lint_err<T: LintRule + 'static>(source: &str, col: usize) {
  assert_lint_err_on_line::<T>(source, 1, col)
}

pub fn assert_lint_err_on_line<T: LintRule + 'static>(
  source: &str,
  line: usize,
  col: usize,
) {
  let rule = T::new();
  let rule_code = rule.code();
  let diagnostics = lint(rule, source);
  assert!(diagnostics.len() == 1);
  assert_diagnostic(&diagnostics[0], rule_code, line, col);
}

pub fn assert_lint_err_n<T: LintRule + 'static>(
  source: &str,
  expected: Vec<usize>,
) {
  let mut real: Vec<(usize, usize)> = Vec::new();
  for col in expected {
    real.push((1, col));
  }
  assert_lint_err_on_line_n::<T>(source, real)
}

pub fn assert_lint_err_on_line_n<T: LintRule + 'static>(
  source: &str,
  expected: Vec<(usize, usize)>,
) {
  let rule = T::new();
  let rule_code = rule.code();
  let diagnostics = lint(rule, source);
  assert!(diagnostics.len() == expected.len());
  for i in 0..diagnostics.len() {
    let (line, col) = expected[i];
    assert_diagnostic(&diagnostics[i], rule_code, line, col);
  }
}
