// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_types)]

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
mod test_util;

pub mod ast_parser;
pub mod context;
mod control_flow;
pub mod diagnostic;
mod globals;
mod handler;
mod ignore_directives;
mod js_regex;
pub mod linter;
pub mod rules;
pub mod swc_util;

pub use deno_ast::view::Program;
pub use deno_ast::view::ProgramRef;

#[cfg(test)]
mod lint_tests {
  use crate::diagnostic::LintDiagnostic;
  use crate::linter::*;
  use crate::rules::{get_recommended_rules, LintRule};
  use crate::test_util::{assert_diagnostic, parse};
  use deno_ast::ParsedSource;

  fn lint(
    source: &str,
    rules: Vec<&'static dyn LintRule>,
  ) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default().rules(rules).build();

    let (_, diagnostics) = linter
      .lint("lint_test.ts".to_string(), source.to_string())
      .expect("Failed to lint");
    diagnostics
  }

  fn lint_with_ast(
    parsed_source: &ParsedSource,
    rules: Vec<&'static dyn LintRule>,
  ) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default().rules(rules).build();

    linter.lint_with_ast(parsed_source)
  }

  fn lint_recommended_rules(source: &str) -> Vec<LintDiagnostic> {
    lint(source, get_recommended_rules())
  }

  fn lint_recommended_rules_with_ast(
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    lint_with_ast(parsed_source, get_recommended_rules())
  }

  fn lint_specified_rule(
    rule: &'static dyn LintRule,
    source: &str,
  ) -> Vec<LintDiagnostic> {
    lint(source, vec![rule])
  }

  #[test]
  fn empty_file() {
    let diagnostics = lint_recommended_rules("");
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn ban_unknown_rule_code() {
    let src = r#"
 // deno-lint-ignore some-rule
 function _foo() {
   // deno-lint-ignore some-rule-2 some-rule-3
   let _bar_foo = true
 }
      "#;
    let diagnostics = lint_recommended_rules(src);

    assert_diagnostic(&diagnostics[0], "ban-unknown-rule-code", 2, 1, src);
    assert_diagnostic(&diagnostics[1], "ban-unknown-rule-code", 4, 3, src);
  }

  #[test]
  fn dont_run_ban_unknown_rule_code_when_no_rules_provided() {
    let src = r#"
 // deno-lint-ignore some-rule
 function _foo() {
   // deno-lint-ignore some-rule-2 some-rule-3
   let _bar_foo = true
 }
      "#;
    let diagnostics = lint(src, vec![]);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn global_ignore_ban_unknown_rule_code() {
    let src = r#"
// deno-lint-ignore-file ban-unknown-rule-code

// deno-lint-ignore some-rule
export function foo() {
  return true
}
      "#;
    let diagnostics = lint_recommended_rules(src);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn unknown_rules_always_know_available_rules() {
    use crate::rules::camelcase::Camelcase;
    let diagnostics = lint_specified_rule(
      &Camelcase,
      r#"
// deno-lint-ignore no-explicit-any
const fooBar: any = 42;
      "#,
    );

    assert!(diagnostics.is_empty());
  }

  #[test]
  fn ban_unused_ignore() {
    let src = r#"
 // deno-lint-ignore no-explicit-any
 function _bar(_p: boolean) {
   // deno-lint-ignore no-misused-new eqeqeq
   const _foo = false
 }
      "#;

    let diagnostics = lint_recommended_rules(src);

    assert_eq!(diagnostics.len(), 2);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 1, src);
    assert_diagnostic(&diagnostics[1], "ban-unused-ignore", 4, 3, src);
  }

  #[test]
  fn ban_unused_ignore_not_report_unexecuted_rule() {
    use crate::rules::camelcase::Camelcase;
    let diagnostics = lint_specified_rule(
      &Camelcase,
      r#"
// deno-lint-ignore no-explicit-any
const _fooBar = 42;
      "#,
    );

    assert!(diagnostics.is_empty());
  }

  #[test]
  fn ban_unused_ignore_file_level_ignore_directive() {
    let diagnostics = lint_recommended_rules(
      r#"
// deno-lint-ignore-file ban-unused-ignore

// deno-lint-ignore no-explicit-any
const _foo = 42;
      "#,
    );

    assert!(diagnostics.is_empty());
  }

  #[test]
  fn ban_unused_ignore_line_level_ignore_directive() {
    // `ban-unused-ignore` can't be ignored via line-level ignore directives
    let src = r#"
// deno-lint-ignore no-explicit-any ban-unused-ignore
const _foo = 42;
      "#;
    let diagnostics = lint_recommended_rules(src);

    assert_eq!(diagnostics.len(), 2);

    // Both `no-explicit-any` and `ban-unused-ignore` are considered "unused"
    // ignore directives in this case. Remember that `ban-unused-ignore`, if
    // it's ignored at a line level, doesn't have any effect.
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 0, src);
    assert_diagnostic(&diagnostics[1], "ban-unused-ignore", 2, 0, src);
  }

  #[test]
  fn file_directive_with_code() {
    let diagnostics = lint_recommended_rules(
      r#"
 // deno-lint-ignore-file no-explicit-any

 function _bar(_p: any) {
   // pass
 }
      "#,
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn file_directive_with_code_unused() {
    let src = r#"
 // deno-lint-ignore-file no-explicit-any no-empty

 function _bar(_p: any) {
   // pass
 }
      "#;
    let diagnostics = lint_recommended_rules(src);

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 1, src);
  }

  #[test]
  fn file_directive_with_code_higher_precedence() {
    let src = r#"
 // deno-lint-ignore-file no-explicit-any

 // deno-lint-ignore no-explicit-any
 function _bar(_p: any) {
   // pass
 }
      "#;
    let diagnostics = lint_recommended_rules(src);

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 4, 1, src);
  }

  #[test]
  fn empty_file_with_ast() {
    let parsed_source = parse("");
    let diagnostics = lint_recommended_rules_with_ast(&parsed_source);
    assert!(diagnostics.is_empty());
  }
}
