// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

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
mod performance_mark;
pub mod rules;
pub mod swc_util;
pub mod tags;

pub use deno_ast::view::Program;
pub use deno_ast::view::ProgramRef;

#[cfg(test)]
mod lint_tests {
  use std::collections::HashSet;

  use crate::diagnostic::LintDiagnostic;
  use crate::linter::*;
  use crate::rules::{get_all_rules, recommended_rules, LintRule};
  use crate::test_util::{assert_diagnostic, parse};
  use deno_ast::ParsedSource;
  use deno_ast::{MediaType, ModuleSpecifier};

  fn lint(
    source: &str,
    rules: Vec<Box<dyn LintRule>>,
    all_rule_codes: HashSet<&'static str>,
  ) -> Vec<LintDiagnostic> {
    let linter = Linter::new(LinterOptions {
      rules,
      all_rule_codes,
      custom_ignore_diagnostic_directive: None,
      custom_ignore_file_directive: None,
    });

    let (_, diagnostics) = linter
      .lint_file(LintFileOptions {
        specifier: ModuleSpecifier::parse("file:///lint_test.ts").unwrap(),
        source_code: source.to_string(),
        media_type: MediaType::TypeScript,
        config: LintConfig {
          default_jsx_factory: None,
          default_jsx_fragment_factory: None,
        },
      })
      .expect("Failed to lint");
    diagnostics
  }

  fn lint_with_ast(
    parsed_source: &ParsedSource,
    rules: Vec<Box<dyn LintRule>>,
    all_rule_codes: HashSet<&'static str>,
  ) -> Vec<LintDiagnostic> {
    let linter = Linter::new(LinterOptions {
      rules,
      all_rule_codes,
      custom_ignore_diagnostic_directive: None,
      custom_ignore_file_directive: None,
    });
    linter.lint_with_ast(
      parsed_source,
      LintConfig {
        default_jsx_factory: None,
        default_jsx_fragment_factory: None,
      },
    )
  }

  fn lint_recommended_rules(source: &str) -> Vec<LintDiagnostic> {
    lint(
      source,
      recommended_rules(get_all_rules()),
      get_all_rules()
        .into_iter()
        .map(|rule| rule.code())
        .collect(),
    )
  }

  fn lint_recommended_rules_with_ast(
    parsed_source: &ParsedSource,
  ) -> Vec<LintDiagnostic> {
    lint_with_ast(
      parsed_source,
      recommended_rules(get_all_rules()),
      get_all_rules()
        .into_iter()
        .map(|rule| rule.code())
        .collect(),
    )
  }

  fn lint_specified_rule(
    rule: Box<dyn LintRule>,
    source: &str,
  ) -> Vec<LintDiagnostic> {
    lint(
      source,
      vec![rule],
      get_all_rules()
        .into_iter()
        .map(|rule| rule.code())
        .collect(),
    )
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
    let diagnostics = lint(src, vec![], HashSet::new());
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
      Box::new(Camelcase),
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
      Box::new(Camelcase),
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
