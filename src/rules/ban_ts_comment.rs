// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::sync::Arc;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;

pub struct BanTsComment;

impl BanTsComment {
  fn lint_comment(&self, context: &Context, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    lazy_static! {
      static ref BTC_REGEX: regex::Regex =
        regex::Regex::new(r#"^/*\s*@ts-(expect-error|ignore|nocheck)$"#)
          .unwrap();
    }

    if BTC_REGEX.is_match(&comment.text) {
      context.add_diagnostic(
        comment.span,
        "ban-ts-comment",
        "ts directives are not allowed",
      );
    }
  }
}

impl LintRule for BanTsComment {
  fn new() -> Box<Self> {
    Box::new(BanTsComment)
  }

  fn code(&self) -> &'static str {
    "ban-ts-comment"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    _module: &swc_ecmascript::ast::Module,
  ) {
    context.leading_comments.values().for_each(|comments| {
      for comment in comments {
        self.lint_comment(&context, comment);
      }
    });
    context.trailing_comments.values().for_each(|comments| {
      for comment in comments {
        self.lint_comment(&context, comment);
      }
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ban_ts_comment_valid() {
    assert_lint_ok_n::<BanTsComment>(vec![
      r#"// just a comment containing @ts-expect-error somewhere"#,
      r#"/* @ts-expect-error */"#,
      r#"/** @ts-expect-error */"#,
      r#"/*
// @ts-expect-error in a block
*/
"#,
    ]);

    assert_lint_ok_n::<BanTsComment>(vec![
      r#"// just a comment containing @ts-ignore somewhere"#,
      r#"/* @ts-ignore */"#,
      r#"/** @ts-ignore */"#,
      r#"/*
// @ts-ignore in a block
*/
"#,
    ]);

    assert_lint_ok_n::<BanTsComment>(vec![
      r#"// just a comment containing @ts-nocheck somewhere"#,
      r#"/* @ts-nocheck */"#,
      r#"/** @ts-nocheck */"#,
      r#"/*
// @ts-nocheck in a block
*/
"#,
    ]);

    assert_lint_ok_n::<BanTsComment>(vec![
      r#"// just a comment containing @ts-check somewhere"#,
      r#"/* @ts-check */"#,
      r#"/** @ts-check */"#,
      r#"/*
// @ts-check in a block
*/
"#,
    ]);

    assert_lint_ok::<BanTsComment>(
      r#"if (false) {
// @ts-ignore: Unreachable code error
console.log('hello');
}"#,
    );
    assert_lint_ok::<BanTsComment>(
      r#"if (false) {
// @ts-expect-error: Unreachable code error
console.log('hello');
}"#,
    );
    assert_lint_ok::<BanTsComment>(
      r#"if (false) {
// @ts-nocheck: Unreachable code error
console.log('hello');
}"#,
    );

    assert_lint_ok::<BanTsComment>(
      r#"// @ts-expect-error: Suppress next line"#,
    );
    assert_lint_ok::<BanTsComment>(r#"// @ts-ignore: Suppress next line"#);
    assert_lint_ok::<BanTsComment>(r#"// @ts-nocheck: Suppress next line"#);
  }

  #[test]
  fn ban_ts_comment_invalid() {
    assert_lint_err::<BanTsComment>(r#"// @ts-expect-error"#, 0);
    assert_lint_err::<BanTsComment>(r#"// @ts-ignore"#, 0);
    assert_lint_err::<BanTsComment>(r#"// @ts-nocheck"#, 0);
  }
}
