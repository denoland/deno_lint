// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::Program;
use deno_ast::swc::common::comments::{Comment, CommentKind};
use deno_ast::{SourceRange, SourceRangedForSpanned};
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct BanUntaggedDeprecation;

const CODE: &str = "ban-untagged-deprecation";
const MESSAGE: &str = "The @deprecated tag must include descriptive text";
const HINT: &str =
  "Provide additional context for the @deprecated tag, e.g., '@deprecated since v2.0'";

impl LintRule for BanUntaggedDeprecation {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    _program: Program,
  ) {
    context
      .all_comments()
      .flat_map(extract_violated_deprecation_ranges)
      .for_each(|range| {
        context.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT)
      });
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_untagged_deprecation.md")
  }
}

/// Returns the ranges of invalid `@deprecated` comments in the given comment.
fn extract_violated_deprecation_ranges(comment: &Comment) -> Vec<SourceRange> {
  if !is_jsdoc_comment(comment) {
    return Vec::new();
  }

  static INVALID_DEPRECATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(?:.*\s+|\s*\*\s*)(@deprecated\s*?)$").unwrap()
  });
  static BLOCK_COMMENT_OPEN_OFFSET: usize = 2; // Length of the "/*".

  INVALID_DEPRECATION_REGEX
    .captures_iter(&comment.text)
    .filter_map(|caps| caps.get(1))
    .map(|mat| {
      let start = comment.start() + mat.start() + BLOCK_COMMENT_OPEN_OFFSET;
      let end = comment.start() + mat.end() + BLOCK_COMMENT_OPEN_OFFSET;
      SourceRange::new(start, end)
    })
    .collect()
}

/// Checks if the given comment is a JSDoc-style comment.
fn is_jsdoc_comment(comment: &Comment) -> bool {
  comment.kind == CommentKind::Block && comment.text.starts_with('*')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_untagged_deprecation_valid() {
    assert_lint_ok! {
    BanUntaggedDeprecation,
    // @deprecated tag with additional context is valid.
    r#"/** @deprecated since v2.0 */"#,
    // @deprecated tag in the middle of comments with additional context is valid.
    r#"/**
 * @param foo - The input value.
 * @public @deprecated since v2.0
 * @returns The computed result.
 */"#,
    // Line comments are not checked.
    r#"// @deprecated "#,
    // Non-JSDoc block comments are not checked.
    r#"/* @deprecated */"#,
    // More than two stars before @deprecated are not treated as JSDoc tag.
    r#"/***@deprecated
 **@deprecated
 ***@deprecated
 */"#,
    // Invalid JSDoc tags are not treated as @deprecated.
    r#"/** @deprecatedtmp */"#,
    r#"/** tmp@deprecated */"#,
         };
  }

  #[test]
  fn ban_untagged_deprecation_invalid() {
    assert_lint_err! {
      BanUntaggedDeprecation,
      // @deprecated tag without additional texts is invalid.
      r#"/** @deprecated */"#: [{ col: 4, line: 1, message: MESSAGE, hint: HINT }],
      r#"/**
 *@deprecated
 */"#: [{ col: 2, line: 2, message: MESSAGE, hint: HINT }],
      r#"/**
 * @deprecated
 */"#: [{ col: 3, line: 2, message: MESSAGE, hint: HINT }],
      r#"/**
@deprecated
*/"#: [{ col: 0, line: 2, message: MESSAGE, hint: HINT }],
      r#"/**
   @deprecated
 */"#: [{ col: 3, line: 2, message: MESSAGE, hint: HINT }],
      r#"/**
 * This function is @deprecated
 */"#: [{ col: 20, line: 2, message: MESSAGE, hint: HINT }],
      // Multiple violations in a single JSDoc comment.
      r#"/**
* @deprecated
* @deprecated
*/"#: [
        { col: 2, line: 2, message: MESSAGE, hint: HINT },
        { col: 2, line: 3, message: MESSAGE, hint: HINT },
      ],
    }
  }
}
