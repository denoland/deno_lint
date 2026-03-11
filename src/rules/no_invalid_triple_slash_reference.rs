// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Comment, CommentKind, Program};
use deno_ast::oxc::span::Span;
use deno_ast::MediaType;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct NoInvalidTripleSlashReference;

const CODE: &str = "no-invalid-triple-slash-reference";

impl LintRule for NoInvalidTripleSlashReference {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    _program: &Program<'a>,
  ) {
    let is_js_like =
      matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx);

    let source_text = context.source_text().to_string();

    for comment in context.all_comments() {
      let comment_text = {
        let span = comment.content_span();
        &source_text[span.start as usize..span.end as usize]
      };
      if let Some(report_kind) =
        check_comment(comment, comment_text, is_js_like)
      {
        context.add_diagnostic_with_hint(
          report_kind.span(),
          CODE,
          report_kind.as_message(),
          report_kind.as_hint(),
        );
      }
    }
  }
}

#[derive(Debug, Eq, PartialEq)]
enum ReportKind {
  /// In JavaScript files, the directives other than `types`, `path` and `lib` are not allowed. This variant
  /// represents such invalid directives in JavaScript.
  InvalidDirectiveInJs(Span),

  /// Represents an unsupported or badly-formed directive
  InvalidDirective(Span),
}

impl ReportKind {
  fn as_message(&self) -> &'static str {
    use ReportKind::*;
    match *self {
      InvalidDirectiveInJs(_) => {
        "This triple-slash reference directive is not allowed in JavaScript"
      }
      InvalidDirective(_) => {
        "Invalid format of triple-slash reference directive"
      }
    }
  }

  fn as_hint(&self) -> &'static str {
    use ReportKind::*;
    match *self {
      InvalidDirectiveInJs(_) => {
        r#"In JavaScript only the `lib`, `path` and `types` directives are allowed, like `/// <reference lib="..." />` or `/// <reference path="..." />` or `/// <reference types="..." />`"#
      }
      InvalidDirective(_) => {
        r#"Correct format is `/// <reference xxx="some_value" />` where `xxx` is one of `types`, `path`, `lib`, or `no-default-lib`"#
      }
    }
  }

  fn span(&self) -> Span {
    use ReportKind::*;
    match *self {
      InvalidDirectiveInJs(span) => span,
      InvalidDirective(span) => span,
    }
  }
}

// These regexes should be consistent with how Deno resolves modules.
// https://github.com/denoland/deno/blob/76e2edc7e1868d7768e259aacbb9a991e1afc462/cli/module_graph.rs
static TRIPLE_SLASH_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"(?i)^/\s*<reference\s.*?/>").unwrap());
static PATH_REFERENCE_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r#"(?i)\spath\s*=\s*["'](?P<value>[^"']*)["']"#).unwrap()
});
static TYPES_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"(?i)\stypes\s*=\s*["']([^"']*)["']"#).unwrap());
static LIB_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"(?i)\slib\s*=\s*["']([^"']*)["']"#).unwrap());
static NO_DEFAULT_LIB_REFERENCE_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r#"(?i)\sno-default-lib\s*=\s*["']([^"']*)["']"#).unwrap()
});

/// Returns `Some` if the comment should be reported.
fn check_comment(
  comment: &Comment,
  comment_text: &str,
  is_js_like: bool,
) -> Option<ReportKind> {
  if matches!(comment.kind, CommentKind::SingleLineBlock | CommentKind::MultiLineBlock) {
    return None;
  }
  if !TRIPLE_SLASH_REFERENCE_RE.is_match(comment_text) {
    return None;
  }

  if is_js_like {
    // In JavaScript, only the `lib`, `no-default-lib`, `path` and `types` directives are allowed
    if is_types_ref(comment_text)
      || is_lib_ref(comment_text)
      || is_path_ref(comment_text)
      || is_no_default_lib_ref(comment_text)
    {
      None
    } else {
      Some(ReportKind::InvalidDirectiveInJs(comment.span))
    }
  } else if is_path_ref(comment_text)
    || is_types_ref(comment_text)
    || is_lib_ref(comment_text)
    || is_no_default_lib_ref(comment_text)
  {
    None
  } else {
    Some(ReportKind::InvalidDirective(comment.span))
  }
}

fn is_path_ref(s: &str) -> bool {
  PATH_REFERENCE_RE.is_match(s)
}

fn is_types_ref(s: &str) -> bool {
  TYPES_REFERENCE_RE.is_match(s)
}

fn is_lib_ref(s: &str) -> bool {
  LIB_REFERENCE_RE.is_match(s)
}

fn is_no_default_lib_ref(s: &str) -> bool {
  NO_DEFAULT_LIB_REFERENCE_RE.is_match(s)
}

#[cfg(test)]
mod tests {
  use deno_ast::oxc::span::Span;

  use super::*;

  fn dummy_span() -> Span {
    Span::new(0, 0)
  }

  fn dummy_comment(kind: CommentKind, text: &str) -> (Comment, String) {
    let span = dummy_span();
    // In OXC, Comment content_span is separate from the comment span.
    // For testing the check_comment function directly, we pass the text separately.
    let comment = Comment::new(span.start, span.end, kind);
    (comment, text.to_string())
  }

  #[test]
  fn test_is_path_ref() {
    let testcases = [
      (r#"/ <reference path="foo" />"#, true),
      (r#"/ <reference path='foo' />"#, true),
      (r#"/ <reference path = "foo" />"#, true),
      (r#"/ <reference ppath = "foo" />"#, false),
    ];

    for (input, expected) in &testcases {
      assert_eq!(*expected, is_path_ref(input));
    }
  }

  #[test]
  fn test_is_types_ref() {
    let testcases = [
      (r#"/ <reference types="foo" />"#, true),
      (r#"/ <reference types='foo' />"#, true),
      (r#"/ <reference types = "foo" />"#, true),
      (r#"/ <reference ttypes = "foo" />"#, false),
    ];

    for (input, expected) in &testcases {
      assert_eq!(*expected, is_types_ref(input));
    }
  }

  #[test]
  fn test_is_lib_ref() {
    let testcases = [
      (r#"/ <reference lib="foo" />"#, true),
      (r#"/ <reference lib='foo' />"#, true),
      (r#"/ <reference lib = "foo" />"#, true),
      (r#"/ <reference llib = "foo" />"#, false),
    ];

    for (input, expected) in &testcases {
      assert_eq!(*expected, is_lib_ref(input));
    }
  }

  #[test]
  fn test_is_no_default_lib_ref() {
    let testcases = [
      (r#"/ <reference no-default-lib="foo" />"#, true),
      (r#"/ <reference no-default-lib='foo' />"#, true),
      (r#"/ <reference no-default-lib = "foo" />"#, true),
      (r#"/ <reference nno-default-lib = "foo" />"#, false),
    ];

    for (input, expected) in &testcases {
      assert_eq!(*expected, is_no_default_lib_ref(input));
    }
  }

  fn line(text: &str) -> (Comment, String) {
    dummy_comment(CommentKind::Line, text)
  }
  fn block(text: &str) -> (Comment, String) {
    dummy_comment(CommentKind::MultiLineBlock, text)
  }

  #[test]
  fn test_check_comment_js() {
    let valid_comments = [
      line(r#"/ <reference types="./mod.d.ts" />"#),
      line(r#"/<reference types="./mod.d.ts" />"#),
      line(r#"/      <reference types="./mod.d.ts"     />           foo bar "#),
      line(r#"/ <reference lib="./mod.d.ts" />"#),
      // normal comment because of inserted "foo"
      line(r#"/ foo <reference path="./mod.d.ts" />"#),
      // normal comment because of inserted "foo"
      line(r#"/ foo <reference lib="./mod.d.ts" />"#),
      // just double slash
      line(r#"<reference path="./mod.d.ts" />"#),
      // just double slash
      line(r#"<reference lib="./mod.d.ts" />"#),
      // just double slash
      line(r#"<reference no-default-lib="true" />"#),
      // block comment
      block(r#"<reference path="./mod.d.ts" />"#),
      // block comment
      block(r#"<reference lib="./mod.d.ts" />"#),
      // block comment
      block(r#"<reference no-default-lib="true" />"#),
    ];
    for (comment, text) in &valid_comments {
      assert!(check_comment(comment, text, true).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference foo="./mod.d.ts" />"#),
      line(r#"/<reference bar />"#),
    ];
    for (comment, text) in &invalid_comments {
      let report_kind = check_comment(comment, text, true).unwrap();
      assert_eq!(report_kind, ReportKind::InvalidDirectiveInJs(dummy_span()))
    }
  }

  #[test]
  fn test_check_comment_not_js() {
    let valid_comments = [
      line(r#"/ <reference types="./mod.d.ts" />"#),
      line(r#"/ <reference path="./mod.d.ts" />"#),
      line(r#"/ <reference lib="./mod.d.ts" />"#),
      line(r#"/ <reference no-default-lib="true" />"#),
      line(r#"/<reference types="./mod.d.ts" />"#),
      line(r#"/      <reference types="./mod.d.ts"     />           foo bar "#),
      line(r#"/ foo <reference path="./mod.d.ts" />"#),
      line(r#"<reference path="./mod.d.ts" />"#),
      block(r#"<reference path="./mod.d.ts" />"#),
    ];
    for (comment, text) in &valid_comments {
      assert!(check_comment(comment, text, false).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference foo="./mod.d.ts" />"#),
      line(r#"/<reference bar />"#),
    ];
    for (comment, text) in &invalid_comments {
      let report_kind = check_comment(comment, text, false).unwrap();
      assert_eq!(report_kind, ReportKind::InvalidDirective(dummy_span()))
    }
  }

  #[test]
  fn triple_slash_reference_valid() {
    // JavaScript
    assert_lint_ok! {
      NoInvalidTripleSlashReference,
      filename: "file:///foo.js",
      r#"/// <reference types="./mod.d.ts" />"#,
      r#"/// <reference lib="lib" />"#,
      r#"/// <reference path="path" />"#,
      r#"// <reference path="path" />"#,
      r#"// <reference lib="lib" />"#,
      r#"/// <reference no-default-lib="true" />"#,
      r#"/* <reference path="path" /> */"#,
      r#"/* <reference lib="lib" /> */"#,
      r#"/* <reference no-default-lib="true" /> */"#,
      r#"/// hello <reference foo="./mod.d.ts" />"#,
      r#"
        /*
        /// <reference foo="foo" />
        */
      "#,
    }

    // TypeScript
    assert_lint_ok! {
      NoInvalidTripleSlashReference,
      filename: "file:///foo.ts",
      r#"/// <reference types="./mod.d.ts" />"#,
      r#"/// <reference path="path" />"#,
      r#"/// <reference lib="lib" />"#,
      r#"/// foo <reference bar />"#,

      // https://github.com/denoland/deno_lint/issues/718
      r#"/// <reference no-default-lib="true" />"#,
    };
  }

  #[test]
  fn triple_slash_reference_invalid() {
    let (not_types_in_js_msg, not_types_in_js_hint) = {
      let r = ReportKind::InvalidDirectiveInJs(dummy_span());
      (r.as_message(), r.as_hint())
    };
    let (invalid_directive_msg, invalid_directive_hint) = {
      let r = ReportKind::InvalidDirective(dummy_span());
      (r.as_message(), r.as_hint())
    };

    // JavaScript
    assert_lint_err! {
      NoInvalidTripleSlashReference,
      filename: "file:///foo.js",
      r#"/// <reference foo />"#: [
        {
          line: 1,
          col: 0,
          message: not_types_in_js_msg,
          hint: not_types_in_js_hint,
        },
      ],
    };

    // TypeScript
    assert_lint_err! {
      NoInvalidTripleSlashReference,
      filename: "file:///foo.ts",
      r#"/// <reference foo />"#: [
        {
          line: 1,
          col: 0,
          message: invalid_directive_msg,
          hint: invalid_directive_hint,
        },
      ],
      r#"/// <reference foo="bar" />"#: [
        {
          line: 1,
          col: 0,
          message: invalid_directive_msg,
          hint: invalid_directive_hint,
        },
      ],
      r#"/// <reference />"#: [
        {
          line: 1,
          col: 0,
          message: invalid_directive_msg,
          hint: invalid_directive_hint,
        },
      ],
    };
  }
}
