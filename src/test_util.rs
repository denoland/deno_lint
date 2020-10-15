// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use crate::diagnostic::LintDiagnostic;
use crate::linter::LinterBuilder;
use crate::rules::LintRule;
use crate::swc_util;
use swc_ecmascript::ast::Module;

// TODO(magurotuna): rename this macro after replacing existing tests with this macro
#[macro_export]
macro_rules! assert_lint_ok_macro {
  ($rule:ty, $src:literal $(,)?) => {
    $crate::test_util::assert_lint_ok::<$rule>($src);
  };
  ($rule:ty, [$($src:literal),* $(,)?] $(,)?) => {
    $(
      $crate::test_util::assert_lint_ok::<$rule>($src);
    )*
  };
}

fn lint(rule: Box<dyn LintRule>, source: &str) -> Vec<LintDiagnostic> {
  let mut linter = LinterBuilder::default()
    .lint_unused_ignore_directives(false)
    .lint_unknown_rules(false)
    .syntax(swc_util::get_default_ts_config())
    .rules(vec![rule])
    .build();

  linter
    .lint("deno_lint_test.tsx".to_string(), source.to_string())
    .expect("Failed to lint")
}

pub fn assert_diagnostic(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
  source: &str,
) {
  if diagnostic.code == code
    && diagnostic.range.start.line == line
    && diagnostic.range.start.col == col
  {
    return;
  }
  panic!(format!(
    "expect diagnostics {} at {}:{} to be {} at {}:{}\n\nsource:\n{}\n",
    diagnostic.code,
    diagnostic.range.start.line,
    diagnostic.range.start.col,
    code,
    line,
    col,
    source,
  ))
}

pub fn assert_lint_ok<T: LintRule + 'static>(source: &str) {
  let rule = T::new();
  let diagnostics = lint(rule, source);
  if !diagnostics.is_empty() {
    panic!(
      "Unexpected diagnostics found:\n{:#?}\n\nsource:\n{}\n",
      diagnostics, source
    );
  }
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
  assert_eq!(
    diagnostics.len(),
    1,
    "1 diagnostic expected, but got {}.\n\nsource:\n{}\n",
    diagnostics.len(),
    source
  );
  assert_diagnostic(&diagnostics[0], rule_code, line, col, source);
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
  assert_eq!(
    diagnostics.len(),
    expected.len(),
    "{} diagnostics expected, but got {}.\n\nsource:\n{}\n",
    expected.len(),
    diagnostics.len(),
    source
  );
  for i in 0..diagnostics.len() {
    let (line, col) = expected[i];
    assert_diagnostic(&diagnostics[i], rule_code, line, col, source);
  }
}

pub fn parse(source_code: &str) -> Module {
  let ast_parser = swc_util::AstParser::new();
  let syntax = swc_util::get_default_ts_config();
  let (parse_result, _comments) =
    ast_parser.parse_module("file_name.ts", syntax, source_code);
  parse_result.unwrap()
}
