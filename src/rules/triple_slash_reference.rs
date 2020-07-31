// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use crate::swc_ecma_ast;

use std::sync::Arc;

pub struct TripleSlashReference;

impl TripleSlashReference {
  fn lint_comment(&self, context: &Context, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    lazy_static! {
      static ref TSR_REGEX: regex::Regex = regex::Regex::new(
        r#"^/\s*<reference\s*(types|path|lib)\s*=\s*["|'](.*)["|']"#
      )
      .unwrap();
    }

    if TSR_REGEX.is_match(&comment.text) {
      context.add_diagnostic(
        comment.span,
        "triple-slash-reference",
        "`triple slash reference` is not allowed",
      );
    }
  }
}

impl LintRule for TripleSlashReference {
  fn new() -> Box<Self> {
    Box::new(TripleSlashReference)
  }

  fn code(&self) -> &'static str {
    "triple-slash-reference"
  }

  fn lint_module(&self, context: Arc<Context>, _module: &swc_ecma_ast::Module) {
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
  fn triple_slash_reference_valid() {
    assert_lint_ok::<TripleSlashReference>(
      r#"
      // <reference path="foo" />
      // <reference types="bar" />
      // <reference lib="baz" />
      import * as foo from 'foo';
      import * as bar from 'bar';
      import * as baz from 'baz';
      "#,
    );

    assert_lint_ok::<TripleSlashReference>(
      r#"
        // <reference path="foo" />
        // <reference types="bar" />
        // <reference lib="baz" />
        import foo = require('foo');
        import bar = require('bar');
        import baz = require('baz');"#,
    );

    assert_lint_ok::<TripleSlashReference>(
      r#"
        /*
        /// <reference types="foo" />
        */
        import * as foo from 'foo';"#,
    );
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
