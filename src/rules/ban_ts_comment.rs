// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::Span;

/// This rule differs from typescript-eslint. In typescript-eslint the following
/// defaults apply:
/// - ts-expect-error: allowed with comment
/// - ts-ignore: not allowed
/// - ts-nocheck: not allowed
///
/// This rules defaults:
/// - ts-expect-error: allowed with comment
/// - ts-ignore: allowed with comment
/// - ts-nocheck: allowed with comment
pub struct BanTsComment;

const CODE: &str = "ban-ts-comment";

#[derive(Clone, Copy)]
enum DirectiveKind {
  ExpectError,
  Ignore,
  Nocheck,
}

impl DirectiveKind {
  fn as_message(&self) -> &'static str {
    use DirectiveKind::*;
    match *self {
      ExpectError => "`@ts-expect-error` is not allowed without comment",
      Ignore => "`@ts-ignore` is not allowed without comment",
      Nocheck => "`@ts-nocheck` is not allowed without comment",
    }
  }

  fn as_hint(&self) -> &'static str {
    use DirectiveKind::*;
    match *self {
      ExpectError => "Add an in-line comment explaining the reason for using `@ts-expect-error`, like `// @ts-expect-error: <reason>`",
      Ignore => "Add an in-line comment explaining the reason for using `@ts-ignore`, like `// @ts-ignore: <reason>`",
      Nocheck => "Add an in-line comment explaining the reason for using `@ts-nocheck`, like `// @ts-nocheck: <reason>`",
    }
  }
}

impl BanTsComment {
  fn report(&self, context: &mut Context, span: Span, kind: DirectiveKind) {
    context.add_diagnostic_with_hint(
      span,
      CODE,
      kind.as_message(),
      kind.as_hint(),
    );
  }
}

impl LintRule for BanTsComment {
  fn new() -> Box<Self> {
    Box::new(BanTsComment)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    _program: dprint_swc_ecma_ast_view::Program,
  ) {
    let mut violated_comment_spans = Vec::new();

    violated_comment_spans.extend(context.all_comments().filter_map(|c| {
      let kind = check_comment(c)?;
      Some((c.span, kind))
    }));

    for (span, kind) in violated_comment_spans {
      self.report(context, span, kind);
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_ts_comment.md")
  }
}

/// Returns `None` if the comment includes no directives.
fn check_comment(comment: &Comment) -> Option<DirectiveKind> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  static EXPECT_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^/*\s*@ts-expect-error$"#).unwrap());
  static IGNORE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^/*\s*@ts-ignore$"#).unwrap());
  static NOCHECK_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^/*\s*@ts-nocheck$"#).unwrap());

  if EXPECT_ERROR_REGEX.is_match(&comment.text) {
    return Some(DirectiveKind::ExpectError);
  }
  if IGNORE_REGEX.is_match(&comment.text) {
    return Some(DirectiveKind::Ignore);
  }
  if NOCHECK_REGEX.is_match(&comment.text) {
    return Some(DirectiveKind::Nocheck);
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_ts_comment_valid() {
    assert_lint_ok! {
      BanTsComment,
      r#"// just a comment containing @ts-expect-error somewhere"#,
      r#"/* @ts-expect-error */"#,
      r#"/** @ts-expect-error */"#,
      r#"/*
// @ts-expect-error in a block
*/
"#,
      r#"// just a comment containing @ts-ignore somewhere"#,
      r#"/* @ts-ignore */"#,
      r#"/** @ts-ignore */"#,
      r#"/*
// @ts-ignore in a block
*/
"#,
      r#"// just a comment containing @ts-nocheck somewhere"#,
      r#"/* @ts-nocheck */"#,
      r#"/** @ts-nocheck */"#,
      r#"/*
// @ts-nocheck in a block
*/
"#,
      r#"// just a comment containing @ts-check somewhere"#,
      r#"/* @ts-check */"#,
      r#"/** @ts-check */"#,
      r#"/*
// @ts-check in a block
*/
"#,
      r#"if (false) {
// @ts-ignore: Unreachable code error
console.log('hello');
}"#,
      r#"if (false) {
// @ts-expect-error: Unreachable code error
console.log('hello');
}"#,
      r#"if (false) {
// @ts-nocheck: Unreachable code error
console.log('hello');
}"#,
    };
  }

  #[test]
  fn ban_ts_comment_invalid() {
    assert_lint_err! {
      BanTsComment,
      r#"// @ts-expect-error"#: [
            {
              col: 0,
              message: DirectiveKind::ExpectError.as_message(),
              hint: DirectiveKind::ExpectError.as_hint(),
            }
          ],
      r#"/// @ts-expect-error"#: [
            {
              col: 0,
              message: DirectiveKind::ExpectError.as_message(),
              hint: DirectiveKind::ExpectError.as_hint(),
            }
          ],
      r#"//@ts-expect-error"#: [
            {
              col: 0,
              message: DirectiveKind::ExpectError.as_message(),
              hint: DirectiveKind::ExpectError.as_hint(),
            }
          ],
    r#"// @ts-ignore"#: [
            {
              col: 0,
              message: DirectiveKind::Ignore.as_message(),
              hint: DirectiveKind::Ignore.as_hint(),
            }
          ],
    r#"/// @ts-ignore"#: [
            {
              col: 0,
              message: DirectiveKind::Ignore.as_message(),
              hint: DirectiveKind::Ignore.as_hint(),
            }
          ],
    r#"//@ts-ignore"#: [
            {
              col: 0,
              message: DirectiveKind::Ignore.as_message(),
              hint: DirectiveKind::Ignore.as_hint(),
            }
          ],
    r#"// @ts-nocheck"#: [
            {
              col: 0,
              message: DirectiveKind::Nocheck.as_message(),
              hint: DirectiveKind::Nocheck.as_hint(),
            }
          ],
    r#"/// @ts-nocheck"#: [
            {
              col: 0,
              message: DirectiveKind::Nocheck.as_message(),
              hint: DirectiveKind::Nocheck.as_hint(),
            }
          ],
    r#"//@ts-nocheck"#: [
            {
              col: 0,
              message: DirectiveKind::Nocheck.as_message(),
              hint: DirectiveKind::Nocheck.as_hint(),
            }
          ],
    };
  }
}
