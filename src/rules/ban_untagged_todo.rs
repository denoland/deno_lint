// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use regex::Regex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;

pub struct BanUntaggedTodo;

impl BanUntaggedTodo {
  fn lint_comment(&self, context: &Context, comment: &Comment) {
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

    context.add_diagnostic(
      comment.span,
      "banUntaggedTodo",
      "TODO should be tagged with (@username) or (#issue)",
    );
  }
}

impl LintRule for BanUntaggedTodo {
  fn new() -> Box<Self> {
    Box::new(BanUntaggedTodo)
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
