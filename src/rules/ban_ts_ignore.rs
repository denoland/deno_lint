// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use std::sync::Arc;

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
      "ban-ts-ignore",
      "@ts-ignore is not allowed",
    );
  }
}

impl LintRule for BanTsIgnore {
  fn new() -> Box<Self> {
    Box::new(BanTsIgnore)
  }

  fn code(&self) -> &'static str {
    "ban-ts-ignore"
  }

  fn lint_module(&self, context: Arc<Context>, _module: &swc_ecmascript::ast::Module) {
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
  fn ban_ts_ignore() {
    assert_lint_err_on_line::<BanTsIgnore>(
      r#"
// @ts-ignore
function foo() {
  // pass
}
    "#,
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
      3,
      2,
    );
  }
}
