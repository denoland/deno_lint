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
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn ban_ts_ignore() {
    test_lint(
      "ban_ts_ignore",
      r#"
// @ts-ignore
function foo() {
  // pass
}

function bar() {
  // @ts-ignore
  const a = "bar";
}
      "#,
      vec![BanTsIgnore::new()],
      json!([{
        "code": "banTsIgnore",
        "message": "@ts-ignore is not allowed",
        "location": {
          "filename": "ban_ts_ignore",
          "line": 2,
          "col": 0,
        }
      }, {
        "code": "banTsIgnore",
        "message": "@ts-ignore is not allowed",
        "location": {
          "filename": "ban_ts_ignore",
          "line": 8,
          "col": 2,
        }
      }]),
    )
  }
}
