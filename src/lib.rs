// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

mod control_flow;
pub mod diagnostic;
mod globals;
mod js_regex;
pub mod linter;
pub mod rules;
mod scopes;
pub mod swc_util;

#[cfg(test)]
mod test_util;

#[cfg(test)]
mod lint_tests {
  use crate::diagnostic::LintDiagnostic;
  use crate::linter::*;
  use crate::rules::get_recommended_rules;
  use crate::test_util::assert_diagnostic;

  fn lint(
    source: &str,
    unknown_rules: bool,
    unused_dir: bool,
  ) -> Vec<LintDiagnostic> {
    let mut linter = LinterBuilder::default()
      .lint_unknown_rules(unknown_rules)
      .lint_unused_ignore_directives(unused_dir)
      .rules(get_recommended_rules())
      .build();

    linter
      .lint("lint_test.ts".to_string(), source.to_string())
      .expect("Failed to lint")
  }

  #[test]
  fn empty_file() {
    let diagnostics = lint("", true, false);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn warn_unknown_rules() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore some-rule
 function foo() {
   // deno-lint-ignore some-rule-2 some-rule-3
   let bar_foo = true
 }
      "#,
      true,
      false,
    );

    assert_diagnostic(&diagnostics[0], "ban-unknown-rule-code", 2, 1);
    assert_diagnostic(&diagnostics[1], "ban-unknown-rule-code", 4, 3);
  }

  #[test]
  fn ignore_unknown_rules() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore some-rule
 function foo() {
   // pass
 }
      "#,
      false,
      false,
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn warn_unused_dir() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore no-explicit-any
 function bar(p: boolean) {
   // deno-lint-ignore no-misused-new eqeqeq
   let foo_bar = false
 }
      "#,
      false,
      true,
    );

    assert_eq!(diagnostics.len(), 2);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 1);
    assert_diagnostic(&diagnostics[1], "ban-unused-ignore", 4, 3);
  }

  #[test]
  fn ignore_unused_dir() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore no-explicit-any
 function bar(p: boolean) {
   // pass
 }
      "#,
      false,
      false,
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn file_directive_with_code() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore-file no-explicit-any

 function bar(p: any) {
   // pass
 }
      "#,
      false,
      false,
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn file_directive_with_code_unused() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore-file no-explicit-any no-empty

 function bar(p: any) {
   // pass
 }
      "#,
      false,
      true,
    );

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 1);
  }

  #[test]
  fn file_directive_with_code_higher_precedence() {
    let diagnostics = lint(
      r#"
 // deno-lint-ignore-file no-explicit-any

 // deno-lint-ignore no-explicit-any
 function bar(p: any) {
   // pass
 }
      "#,
      false,
      true,
    );

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 4, 1);
  }
}
