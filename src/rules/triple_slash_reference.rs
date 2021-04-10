// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::Span;

pub struct TripleSlashReference;

impl TripleSlashReference {
  fn report(&self, context: &mut Context, span: Span) {
    context.add_diagnostic(
      span,
      "triple-slash-reference",
      "`triple slash reference` is not allowed",
    );
  }
}

impl LintRule for TripleSlashReference {
  fn new() -> Box<Self> {
    Box::new(TripleSlashReference)
  }

  fn code(&self) -> &'static str {
    "triple-slash-reference"
  }

  fn lint_program(&self, context: &mut Context, _program: ProgramRef<'_>) {
    let mut violated_comment_spans = Vec::new();

    violated_comment_spans.extend(context.all_comments().filter_map(|c| {
      if check_comment(c) {
        Some(c.span)
      } else {
        None
      }
    }));

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

  static TSR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^/\s*<reference\s*(types|path|lib)\s*=\s*["|'](.*)["|']"#)
      .unwrap()
  });

  TSR_REGEX.is_match(&comment.text)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn triple_slash_reference_valid() {
    assert_lint_ok! {
      TripleSlashReference,
      r#"
      // <reference path="foo" />
      // <reference types="bar" />
      // <reference lib="baz" />
      import * as foo from 'foo';
      import * as bar from 'bar';
      import * as baz from 'baz';
      "#,
      r#"
        // <reference path="foo" />
        // <reference types="bar" />
        // <reference lib="baz" />
        import foo = require('foo');
        import bar = require('bar');
        import baz = require('baz');"#,
      r#"
        /*
        /// <reference types="foo" />
        */
        import * as foo from 'foo';"#,
    };
  }

  #[test]
  fn triple_slash_reference_invalid() {
    assert_lint_err_on_line::<TripleSlashReference>(
      r#"
/// <reference types="foo" />
import * as foo from 'foo';"#,
      2,
      0,
    );

    assert_lint_err_on_line::<TripleSlashReference>(
      r#"
/// <reference types="foo" />
import foo = require('foo');
    "#,
      2,
      0,
    );

    assert_lint_err::<TripleSlashReference>(
      r#"/// <reference path="foo" />"#,
      0,
    );
    assert_lint_err::<TripleSlashReference>(
      r#"/// <reference types="foo" />"#,
      0,
    );
    assert_lint_err::<TripleSlashReference>(
      r#"/// <reference lib="foo" />"#,
      0,
    );
  }
}
