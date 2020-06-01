// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;

pub struct BanTsIgnore;

impl BanTsIgnore {
  fn lint_comment(&self, context: &Context, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    if !comment.text.contains("@ts-ignore") {
      return;
    }

    context.add_diagnostic(
      comment.span,
      "banTsIgnore",
      "@ts-ignore is not allowed",
    );
  }
}

impl LintRule for BanTsIgnore {
  fn new() -> Box<Self> {
    Box::new(BanTsIgnore)
  }

  fn code(&self) -> &'static str {
    "banTsIgnore"
  }

  fn lint_module(&self, context: Context, _module: swc_ecma_ast::Module) {
    context.leading_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        self.lint_comment(&context, comment);
      }
    });
    context.trailing_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
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
  fn ban_ts_ignore() {
    assert_lint_err_on_line::<BanTsIgnore>(
      r#"
// @ts-ignore
function foo() {
  // pass
}
    "#,
      "banTsIgnore",
      2,
      0,
    );
    assert_lint_err_on_line::<BanTsIgnore>(
      r#"
function bar() {
  // @ts-ignore
  const a = "bar";
}
    "#,
      "banTsIgnore",
      3,
      2,
    );
  }
}
