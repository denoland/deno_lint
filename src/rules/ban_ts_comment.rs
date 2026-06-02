// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{Comment, CommentKind, Program};
use deno_ast::oxc::span::Span;
use once_cell::sync::Lazy;
use regex::Regex;

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
#[derive(Debug)]
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
    let mut violated_comment_ranges = Vec::new();

    violated_comment_ranges.extend(context.all_comments().filter_map(|c| {
      let kind = check_comment(c, context)?;
      Some((c.span, kind))
    }));

    for (span, kind) in violated_comment_ranges {
      self.report(context, span, kind);
    }
  }
}

/// Returns `None` if the comment includes no directives.
fn check_comment(comment: &Comment, ctx: &Context) -> Option<DirectiveKind> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let text = ctx.comment_text(comment);

  static EXPECT_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^/*\s*@ts-expect-error\s*$").unwrap());
  static IGNORE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^/*\s*@ts-ignore\s*$").unwrap());
  static NOCHECK_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^/*\s*@ts-nocheck\s*$").unwrap());

  if EXPECT_ERROR_REGEX.is_match(text) {
    return Some(DirectiveKind::ExpectError);
  }
  if IGNORE_REGEX.is_match(text) {
    return Some(DirectiveKind::Ignore);
  }
  if NOCHECK_REGEX.is_match(text) {
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
      r#"// just a random @ts-expect-error     comment with too many spaces"#,
      r#"/* @ts-expect-error */"#,
      r#"/** @ts-expect-error */"#,
      r#"/*
// @ts-expect-error in a block
*/
"#,
      r#"// just a comment containing @ts-ignore somewhere"#,
      r#"// just a random @ts-ignore     comment with too many spaces"#,
      r#"/* @ts-ignore */"#,
      r#"/** @ts-ignore */"#,
      r#"/*
// @ts-ignore in a block
*/
"#,
      r#"// just a comment containing @ts-nocheck somewhere"#,
      r#"// just a random @ts-nocheck     comment with too many spaces"#,
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
    //@ts-expect-error
    assert_lint_err! {
      BanTsComment,
      DirectiveKind::ExpectError.as_message(),
      DirectiveKind::ExpectError.as_hint(),
      r#"// @ts-expect-error"# : [
        {
          col: 0
        }
      ],
      r#"/// @ts-expect-error"# : [
        {
          col: 0
        }
      ],
      r#"/// @ts-expect-error"# : [
        {
          col: 0
        }
      ],
      r#"// @ts-expect-error    "# : [
        {
          col: 0
        }
      ]
    }

    //@ts-ignore
    assert_lint_err! {
      BanTsComment,
      DirectiveKind::Ignore.as_message(),
      DirectiveKind::Ignore.as_hint(),
      r#"// @ts-ignore"# : [
        {
          col: 0
        }
      ],
      r#"/// @ts-ignore"# : [
        {
          col: 0
        }
      ],
      r#"//@ts-ignore"# : [
        {
          col: 0
        }
      ],
      r#"// @ts-ignore    "# : [
        {
          col: 0
        }
      ]
    }

    //@ts-nocheck
    assert_lint_err! {
      BanTsComment,
      DirectiveKind::Nocheck.as_message(),
      DirectiveKind::Nocheck.as_hint(),
      r#"// @ts-nocheck"# : [
        {
          col: 0
        }
      ],
      r#"/// @ts-nocheck"# : [
        {
          col: 0
        }
      ],
      r#"//@ts-nocheck"# : [
        {
          col: 0
        }
      ],
      r#"// @ts-nocheck    "# : [
        {
          col: 0
        }
      ]
    }
  }
}
