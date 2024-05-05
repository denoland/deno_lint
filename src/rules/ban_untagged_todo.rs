// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::Program;
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::SourceRangedForSpanned;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct BanUntaggedTodo;

const CODE: &str = "ban-untagged-todo";
const MESSAGE: &str = "TODO should be tagged with (@username) or (#issue)";
const HINT: &str = "Add a user tag or issue reference to the TODO comment, e.g. TODO(@djones), TODO(djones), TODO(#123)";

impl LintRule for BanUntaggedTodo {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    _program: Program,
  ) {
    let mut violated_comment_ranges = Vec::new();

    violated_comment_ranges.extend(context.all_comments().filter_map(|c| {
      if check_comment(c) {
        Some(c.range())
      } else {
        None
      }
    }));

    for range in violated_comment_ranges {
      context.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_untagged_todo.md")
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

  static TODO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"todo\((#|@)?\S+\)").unwrap());

  if TODO_RE.is_match(text) {
    return false;
  }

  true
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_ts_ignore_valid() {
    assert_lint_ok! {
      BanUntaggedTodo,
      r#"
// TODO(@someusername)
const c = "c";
      "#,
      r#"
// TODO(@someusername) this should be fixed in next release
const c = "c";
      "#,
      r#"
// TODO(someusername)
const c = "c";
      "#,
      r#"
// TODO(someusername) this should be fixed in next release
const c = "c";
      "#,
      r#"
// TODO(#1234)
const b = "b";
      "#,
      r#"
// TODO(#1234) this should be fixed in next release
const b = "b";
      "#,
    };
  }

  #[test]
  fn ban_ts_ignore_invalid() {
    assert_lint_err! {
      BanUntaggedTodo,
      r#"
// TODO
function foo() {
  // pass
}
      "#: [{ col: 0, line: 2, message: MESSAGE, hint: HINT }],
    r#"
// TODO this should be fixed in next release (username)
const a = "a";
      "#: [{ col: 0, line: 2, message: MESSAGE, hint: HINT }],
    r#"
// TODO this should be fixed in next release (#1234)
const b = "b";
      "#: [{ col: 0, line: 2, message: MESSAGE, hint: HINT }],
    r#"
// TODO this should be fixed in next release (@someusername)
const c = "c";
      "#: [{ col: 0, line: 2, message: MESSAGE, hint: HINT }],
    }
  }
}
