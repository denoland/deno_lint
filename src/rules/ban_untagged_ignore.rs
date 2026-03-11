// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct BanUntaggedIgnore;

const CODE: &str = "ban-untagged-ignore";

impl LintRule for BanUntaggedIgnore {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    _program: &Program<'a>,
  ) {
    let mut violated_ranges: Vec<Span> = context
      .file_ignore_directive()
      .iter()
      .filter(|d| d.ignore_all())
      .map(|d| d.range())
      .collect();

    violated_ranges.extend(
      context
        .line_ignore_directives()
        .values()
        .filter(|d| d.ignore_all())
        .map(|d| d.range()),
    );

    for range in violated_ranges {
      context.add_diagnostic_with_hint(
        range,
        CODE,
        "Ignore directive requires lint rule name(s)",
        "Add one or more lint rule names.  E.g. // deno-lint-ignore adjacent-overload-signatures",
      )
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_untagged_ignore_valid() {
    assert_lint_ok! {
      BanUntaggedIgnore,
      r#"
// deno-lint-ignore no-explicit-any
export const foo: any = 42;
    "#,
    };
  }

  #[test]
  fn ban_untagged_ignore_invalid() {
    assert_lint_err! {
      BanUntaggedIgnore,
      r#"
// deno-lint-ignore
export const foo: any = 42;
      "#: [
        {
          line: 2,
          col: 0,
          message: "Ignore directive requires lint rule name(s)",
          hint: "Add one or more lint rule names.  E.g. // deno-lint-ignore adjacent-overload-signatures",
        }
      ]
    };
  }
}
