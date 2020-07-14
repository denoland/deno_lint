// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::comments::Comment;
use crate::swc_common::comments::CommentKind;
use crate::swc_ecma_ast;
use regex::Regex;

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
      "ban-untagged-todo",
      "TODO should be tagged with (@username) or (#issue)",
    );
  }
}

impl LintRule for BanUntaggedTodo {
  fn new() -> Box<Self> {
    Box::new(BanUntaggedTodo)
  }

  fn code(&self) -> &'static str {
    "ban-untagged-todo"
  }

  fn lint_module(&self, context: Context, _module: &swc_ecma_ast::Module) {
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
    assert_lint_ok_n::<BanUntaggedTodo>(vec![
      r#"
// TODO(#1234)
const b = "b";
      "#,
      r#"
// TODO(@someusername)
const c = "c";
      "#,
    ]);
    assert_lint_err_on_line::<BanUntaggedTodo>(
      r#"
// TODO
function foo() {
  // pass
}
      "#,
      2,
      0,
    );
    assert_lint_err_on_line::<BanUntaggedTodo>(
      r#"
// TODO(username)
const a = "a";
      "#,
      2,
      0,
    );
  }
}
