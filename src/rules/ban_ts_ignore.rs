// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;

pub struct BanTsIgnore {
  context: Context,
}

impl BanTsIgnore {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn lint_comment(&self, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    if !comment.text.contains("@ts-ignore") {
      return;
    }

    self.context.add_diagnostic(
      comment.span,
      "banTsIgnore",
      "Don't use `// @ts-ginore`",
    );
  }

  pub fn lint_comments(&self) {
    self.context.leading_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        self.lint_comment(comment);
      }
    });
    self.context.trailing_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        self.lint_comment(comment);
      }
    });
  }
}
