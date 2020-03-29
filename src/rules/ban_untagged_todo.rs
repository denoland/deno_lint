// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use regex::Regex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;

pub struct BanUntaggedTodo {
  context: Context,
}

impl BanUntaggedTodo {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn lint_comment(&self, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    let comment_text = comment.text.to_lowercase().trim_start().to_string();

    if !comment_text.starts_with("todo") {
      return;
    }

    let re = Regex::new(r#"todo\((#|@)\S+\)"#).unwrap();
    if re.is_match(&comment_text) {
      return;
    }

    self.context.add_diagnostic(
      &comment.span,
      "banUntaggedTodo",
      "TODO should be tagged with (@username) or (#issue)",
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
