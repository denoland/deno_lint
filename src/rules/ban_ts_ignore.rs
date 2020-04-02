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
      "Don't use `// @ts-ginore`",
    );
  }
}

impl LintRule for BanTsIgnore {
  fn new() -> Box<Self> {
    Box::new(BanTsIgnore)
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
