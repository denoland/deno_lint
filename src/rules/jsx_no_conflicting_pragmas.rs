// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct JSXNoConflictingPragmas;

const CODE: &str = "jsx-no-conflicting-pragmas";
const MESSAGE: &str = "Conflicting JSX pragmas";
const HINT: &str = "The classic runtime pragmas `@jsx` and `@jsxFragment` are ignored when `@jsxImportSource` (the automatic runtime) is set. Use one runtime or the other, not both.";

// `@jsxImportSource` selects the automatic JSX runtime. It also matches
// `@jsxImportSourceTypes`.
static IMPORT_SOURCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"@jsxImportSource\b").unwrap());

// `@jsx`, `@jsxFrag` and `@jsxFragment` are classic runtime pragmas. The
// trailing word boundary makes sure we don't match `@jsxImportSource` or
// `@jsxRuntime`.
static CLASSIC_PRAGMA_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"@jsx(Frag(ment)?)?\b").unwrap());

impl LintRule for JSXNoConflictingPragmas {
  fn tags(&self) -> Tags {
    &[tags::REACT, tags::JSX]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    // JSX pragmas only take effect in the file's leading comments, so only
    // consider comments that appear before the first statement.
    let first_item_start = match program {
      Program::Module(module) => module.body.first().map(|n| n.start()),
      Program::Script(script) => script.body.first().map(|n| n.start()),
    };

    let mut has_import_source = false;
    let mut classic_pragma_ranges: Vec<SourceRange> = Vec::new();

    for comment in context.all_comments() {
      if let Some(first_item_start) = first_item_start {
        if comment.start() >= first_item_start {
          continue;
        }
      }

      if IMPORT_SOURCE_RE.is_match(&comment.text) {
        has_import_source = true;
      }

      // A comment can hold the `@jsxImportSource` pragma and still not be a
      // classic pragma, so check both independently.
      if CLASSIC_PRAGMA_RE.is_match(&comment.text) {
        classic_pragma_ranges.push(comment.range());
      }
    }

    if has_import_source {
      for range in classic_pragma_ranges {
        context.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_no_conflicting_pragmas_valid() {
    assert_lint_ok! {
      JSXNoConflictingPragmas,
      filename: "file:///foo.jsx",
      // Only the automatic runtime.
      r#"/** @jsxImportSource https://esm.sh/preact */
    const a = <div />;"#,
      // Only the classic runtime.
      r#"/** @jsx h */
/** @jsxFragment Fragment */
const a = <div />;"#,
      // `@jsxImportSource` paired with `@jsxImportSourceTypes` is fine.
      r#"/** @jsxImportSource https://esm.sh/preact */
/** @jsxImportSourceTypes https://esm.sh/preact */
const a = <div />;"#,
      // No pragmas at all.
      r#"const a = <div />;"#,
    };
  }

  #[test]
  fn jsx_no_conflicting_pragmas_invalid() {
    assert_lint_err! {
      JSXNoConflictingPragmas,
      filename: "file:///foo.jsx",
      r#"/** @jsxImportSource https://esm.sh/preact */
/** @jsx h */
const a = <div />;"#: [
        {
          line: 2,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"/** @jsxImportSource https://esm.sh/preact */
/** @jsx h */
/** @jsxFragment Fragment */
const a = <div />;"#: [
        {
          line: 2,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 3,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      // Order doesn't matter.
      r#"/** @jsx h */
/** @jsxImportSource https://esm.sh/preact */
const a = <div />;"#: [
        {
          line: 1,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
