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
  use dprint_swc_ecma_ast_view::TokenAndSpan;
  use std::rc::Rc;
  use swc_common::comments::SingleThreadedComments;
  use swc_common::SourceMap;
  use swc_ecmascript::ast::Program;

  fn lint(
    source: &str,
    unknown_rules: bool,
    unused_dir: bool,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default()
      .lint_unknown_rules(unknown_rules)
      .lint_unused_ignore_directives(unused_dir)
      .rules(rules)
      .build();

    let (_, diagnostics) = linter
      .lint("lint_test.ts".to_string(), source.to_string())
      .expect("Failed to lint");
    diagnostics
  }

  fn lint_with_ast(
    ast: Program,
    comments: SingleThreadedComments,
    source_map: Rc<SourceMap>,
    tokens: Vec<TokenAndSpan>,
    unknown_rules: bool,
    unused_dir: bool,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    let linter = LinterBuilder::default()
      .lint_unknown_rules(unknown_rules)
      .lint_unused_ignore_directives(unused_dir)
      .rules(rules)
      .build();

    let (_, diagnostics) = linter
      .lint_with_ast(
        "lint_test.ts".to_string(),
        &ast,
        &comments,
        source_map,
        &tokens,
      )
      .expect("Failed to lint");
    diagnostics
  }

  fn lint_recommended_rules(
    source: &str,
    unknown_rules: bool,
    unused_dir: bool,
  ) -> Vec<LintDiagnostic> {
    lint(source, unknown_rules, unused_dir, get_recommended_rules())
  }

  fn lint_recommended_rules_with_ast(
    ast: Program,
    comments: SingleThreadedComments,
    source_map: Rc<SourceMap>,
    tokens: Vec<TokenAndSpan>,
    unknown_rules: bool,
    unused_dir: bool,
  ) -> Vec<LintDiagnostic> {
    lint_with_ast(
      ast,
      comments,
      source_map,
      tokens,
      unknown_rules,
      unused_dir,
      get_recommended_rules(),
    )
  }

  fn lint_specified_rule<T: LintRule + 'static>(
    source: &str,
    unknown_rules: bool,
    unused_dir: bool,
  ) -> Vec<LintDiagnostic> {
    lint(source, unknown_rules, unused_dir, vec![T::new()])
  }

  #[test]
  fn empty_file() {
    let diagnostics = lint_recommended_rules("", true, false);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn warn_unknown_rules() {
    let src = r#"
 // deno-lint-ignore some-rule
 function _foo() {
   // deno-lint-ignore some-rule-2 some-rule-3
   let _bar_foo = true
 }
      "#;
    let diagnostics = lint_recommended_rules(src, true, false);

    assert_diagnostic(&diagnostics[0], "ban-unknown-rule-code", 2, 1, src);
    assert_diagnostic(&diagnostics[1], "ban-unknown-rule-code", 4, 3, src);
  }

  #[test]
  fn ignore_unknown_rules() {
    let diagnostics = lint_recommended_rules(
      r#"
 // deno-lint-ignore some-rule
 function _foo() {
   // pass
 }
      "#,
      false,
      false,
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn unknown_rules_always_know_available_rules() {
    use crate::rules::camelcase::Camelcase;
    let diagnostics = lint_specified_rule::<Camelcase>(
      r#"
// deno-lint-ignore no-explicit-any
const fooBar: any = 42;
      "#,
      true,
      false,
    );

    assert!(diagnostics.is_empty());
  }

  #[test]
  fn ban_unused_ignore_enabled() {
    let src = r#"
 // deno-lint-ignore no-explicit-any
 function _bar(_p: boolean) {
   // deno-lint-ignore no-misused-new eqeqeq
   const _foo = false
 }
      "#;

    let diagnostics = lint_recommended_rules(
      src, false, true, // enables `ban-unused-ignore`
    );

    assert_eq!(diagnostics.len(), 2);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 1, src);
    assert_diagnostic(&diagnostics[1], "ban-unused-ignore", 4, 3, src);
  }

  #[test]
  fn ban_unused_ignore_disabled() {
    let diagnostics = lint_recommended_rules(
      r#"
 // deno-lint-ignore no-explicit-any
 function _bar(_p: boolean) {
   // pass
 }
      "#,
      false,
      false, // disables `ban-unused-ignore`
    );

    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn ban_unused_ignore_not_report_unexecuted_rule() {
    use crate::rules::camelcase::Camelcase;
    let diagnostics = lint_specified_rule::<Camelcase>(
      r#"
// deno-lint-ignore no-explicit-any
const _fooBar = 42;
      "#,
      false,
      true, // enables `ban-unused-ignore`
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
      false,
      true, // enables `ban-unused-ignore`
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
    let diagnostics = lint_recommended_rules(
      src, false, true, // enables `ban-unused-ignore`
    );

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 2, 0, src);
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
      false,
      false,
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
    let diagnostics = lint_recommended_rules(
      src, false, true, // enables `ban-unused-ignore`
    );

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
    let diagnostics = lint_recommended_rules(src, false, true);

    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(&diagnostics[0], "ban-unused-ignore", 4, 1, src);
  }

  #[test]
  fn empty_file_with_ast() {
    let (ast, comments, source_map, tokens) = parse("");
    let diagnostics = lint_recommended_rules_with_ast(
      ast, comments, source_map, tokens, true, false,
    );
    assert!(diagnostics.is_empty());
  }
}
