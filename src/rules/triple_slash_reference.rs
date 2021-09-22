// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::ProgramRef;
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::swc::common::Span;
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

#[derive(Debug)]
pub struct TripleSlashReference;

const CODE: &str = "triple-slash-reference";

#[derive(Display)]
enum TripleSlashReferenceMessage {
  #[display(fmt = "`triple slash reference` is not allowed")]
  Unexpected,
}

impl TripleSlashReference {
  fn report(&self, context: &mut Context, span: Span) {
    context.add_diagnostic(span, CODE, TripleSlashReferenceMessage::Unexpected);
  }
}

impl LintRule for TripleSlashReference {
  fn new() -> Arc<Self> {
    Arc::new(TripleSlashReference)
  }

  fn code(&self) -> &'static str {
    CODE
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

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/triple_slash_reference.md")
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
    assert_lint_err! {
      TripleSlashReference,
      r#"
/// <reference types="foo" />
import * as foo from 'foo';"#:[
      {
        line: 2,
        col: 0,
        message: TripleSlashReferenceMessage::Unexpected,
      }],
      r#"
/// <reference types="foo" />
import foo = require('foo');
    "#:[
      {
        line: 2,
        col: 0,
        message: TripleSlashReferenceMessage::Unexpected,
      }],
      r#"/// <reference path="foo" />"#: [
      {
        col: 0,
        message: TripleSlashReferenceMessage::Unexpected,
      }],
      r#"/// <reference types="foo" />"#: [
      {
        col: 0,
        message: TripleSlashReferenceMessage::Unexpected,
      }],
      r#"/// <reference lib="foo" />"#: [
      {
        col: 0,
        message: TripleSlashReferenceMessage::Unexpected,
      }],
    }
  }
}
