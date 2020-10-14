// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use regex::Regex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::Span;

pub struct BanUntaggedTodo;

impl BanUntaggedTodo {
  fn report(&self, context: &mut Context, span: Span) {
    context.add_diagnostic(
      span,
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

  fn lint_module(
    &self,
    context: &mut Context,
    _module: &swc_ecmascript::ast::Module,
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
}

/// Returns `true` if the comment should be reported.
fn check_comment(comment: &Comment) -> bool {
  if comment.kind != CommentKind::Line {
    return false;
  }

  let text = comment.text.to_lowercase();
  let text = text.trim_start();

  if !text.starts_with("todo") {
    return false;
  }

  lazy_static! {
    static ref TODO_RE: Regex = Regex::new(r#"todo\((#|@)\S+\)"#).unwrap();
  }

  if TODO_RE.is_match(text) {
    return false;
  }

  true
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
