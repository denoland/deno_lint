// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

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

impl BanTsComment {
  fn report(&self, context: &mut Context, span: Span) {
    context.add_diagnostic_with_hint(
      span,
      "ban-ts-comment",
      "ts directives are not allowed without comment",
      "Add an in-line comment explaining the reason for using this directive",
    );
  }
}

impl LintRule for BanTsComment {
  fn new() -> Box<Self> {
    Box::new(BanTsComment)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "ban-ts-comment"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    _program: &swc_ecmascript::ast::Program,
  ) {
    let mut violated_comment_spans = Vec::new();

    violated_comment_spans.extend(
      context.leading_comments.values().flatten().filter_map(|c| {
        if check_comment(c) {
          Some(c.span)
        } else {
          None
        }
      }),
    );
    violated_comment_spans.extend(
      context
        .trailing_comments
        .values()
        .flatten()
        .filter_map(|c| if check_comment(c) { Some(c.span) } else { None }),
    );

    for span in violated_comment_spans {
      self.report(context, span);
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of Typescript directives without a comment.

Typescript directives reduce the effectiveness of the compiler, something which should only be done in exceptional circumstances.  The reason why should be documented in a comment alongside the directive.

### Invalid:
```typescript
// @ts-expect-error
let a: number = "I am a string";
```
```typescript
// @ts-ignore
let a: number = "I am a string";
```
```typescript
// @ts-nocheck
let a: number = "I am a string";
```

### Valid:
```typescript
// @ts-expect-error: Temporary workaround (see ticket #422)
let a: number = "I am a string";
```
```typescript
// @ts-ignore: Temporary workaround (see ticket #422)
let a: number = "I am a string";
```
```typescript
// @ts-nocheck: Temporary workaround (see ticket #422)
let a: number = "I am a string";
```
"#
  }
}

/// Returns `true` if the comment should be reported.
fn check_comment(comment: &Comment) -> bool {
  if comment.kind != CommentKind::Line {
    return false;
  }

  static BTC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^/*\s*@ts-(expect-error|ignore|nocheck)$"#).unwrap()
  });

  BTC_REGEX.is_match(&comment.text)
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
              message: "ts directives are not allowed without comment",
              hint: "Add an in-line comment explaining the reason for using this directive",
            }
          ],
    r#"// @ts-ignore"#: [
            {
              col: 0,
              message: "ts directives are not allowed without comment",
              hint: "Add an in-line comment explaining the reason for using this directive",
            }
          ],
    r#"// @ts-nocheck"#: [
            {
              col: 0,
              message: "ts directives are not allowed without comment",
              hint: "Add an in-line comment explaining the reason for using this directive",
            }
          ]
    };
  }
}
