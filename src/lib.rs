// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

#![deny(warnings)]

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
mod test_util;

pub mod ast_parser;
pub mod context;
// TODO(magurotuna): Making control_flow public is just needed for implementing plugin prototype.
// It will be likely possible to remove `pub` later.
pub mod control_flow;
pub mod diagnostic;
mod globals;
mod handler;
mod ignore_directives;
mod js_regex;
pub mod linter;
pub mod rules;
mod scopes;
pub mod swc_util;

#[cfg(test)]
mod lint_tests {
  use crate::diagnostic::LintDiagnostic;
  use crate::linter::*;
  use crate::rules::{get_recommended_rules, LintRule};
  use crate::test_util::{assert_diagnostic, parse};
  use ast_view::{ProgramRef, TokenAndSpan};
  use swc_common::comments::SingleThreadedCommentsMapInner;

  fn lint(source: &str, rules: Vec<Box<dyn LintRule>>) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default().rules(rules).build();

    let (_, diagnostics) = linter
      .lint("lint_test.ts".to_string(), source.to_string())
      .expect("Failed to lint");
    diagnostics
  }

  fn lint_with_ast(
    source_file: &impl SourceFile,
    ast: ProgramRef,
    leading_comments: &SingleThreadedCommentsMapInner,
    trailing_comments: &SingleThreadedCommentsMapInner,
    tokens: &[TokenAndSpan],
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default().rules(rules).build();

    linter.lint_with_ast(
      "lint_test.ts".to_string(),
      source_file,
      ast,
      leading_comments,
      trailing_comments,
      tokens,
    )
  }

  fn lint_recommended_rules(source: &str) -> Vec<LintDiagnostic> {
    lint(source, get_recommended_rules())
  }

  fn lint_recommended_rules_with_ast(
    source_file: &impl SourceFile,
    ast: ProgramRef,
    leading_comments: &SingleThreadedCommentsMapInner,
    trailing_comments: &SingleThreadedCommentsMapInner,
    tokens: &[TokenAndSpan],
  ) -> Vec<LintDiagnostic> {
    lint_with_ast(
      source_file,
      ast,
      leading_comments,
      trailing_comments,
      tokens,
      get_recommended_rules(),
    )
  }

  fn lint_specified_rule<T: LintRule + 'static>(
    source: &str,
  ) -> Vec<LintDiagnostic> {
    lint(source, vec![T::new()])
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
  fn unknown_rules_always_know_available_rules() {
    use crate::rules::camelcase::Camelcase;
    let diagnostics = lint_specified_rule::<Camelcase>(
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
    let diagnostics = lint_specified_rule::<Camelcase>(
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
    let (source_file, ast, leading_comments, trailing_comments, tokens) =
      parse("");
    let diagnostics = lint_recommended_rules_with_ast(
      &source_file,
      (&ast).into(),
      &leading_comments,
      &trailing_comments,
      &tokens,
    );
    assert!(diagnostics.is_empty());
  }
}
