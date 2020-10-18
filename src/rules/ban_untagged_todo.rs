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
    context.add_diagnostic_with_hint(
      span,
      "ban-untagged-todo",
      "TODO should be tagged with (@username) or (#issue)",
      "Add a user tag, e.g. @djones, or issue reference, e.g. #123, to the TODO comment"
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

  fn docs(&self) -> &'static str {
    r#"Requires TODOs to be annotated with either a user tag (@user) or an issue reference (#issue).

TODOs without reference to a user or an issue become stale with no easy way to get more information.

### Valid:
```typescript
// TODO Improve calc engine (@djones)
export function calcValue(): number { }
```
```typescript
// TODO Improve calc engine (#332)
export function calcValue(): number { }
```

### Invalid:
```typescript
// TODO Improve calc engine
export function calcValue(): number { }
```"#
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
  fn ban_ts_ignore_valid() {
    assert_lint_ok_macro! {
      BanUntaggedTodo,
      r#"
// TODO(#1234)
const b = "b";
      "#,
      r#"
// TODO(@someusername)
const c = "c";
      "#,
    };
  }

  #[test]
  fn ban_ts_ignore_invalid() {
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
