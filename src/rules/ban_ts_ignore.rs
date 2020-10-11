// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::Span;

pub struct BanTsIgnore;

impl BanTsIgnore {
  fn report(&self, context: &mut Context, span: Span) {
    context.add_diagnostic(span, "ban-ts-ignore", "@ts-ignore is not allowed");
  }
}

impl LintRule for BanTsIgnore {
  fn new() -> Box<Self> {
    Box::new(BanTsIgnore)
  }

  fn code(&self) -> &'static str {
    "ban-ts-ignore"
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

  if !comment.text.contains("@ts-ignore") {
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
