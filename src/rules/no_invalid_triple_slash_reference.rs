// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::comments::{Comment, CommentKind};
use deno_ast::{SwcSourceRanged, SourceRange};
use deno_ast::MediaType;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoInvalidTripleSlashReference;

const CODE: &str = "no-invalid-triple-slash-reference";

impl LintRule for NoInvalidTripleSlashReference {
  fn new() -> Arc<Self> {
    Arc::new(NoInvalidTripleSlashReference)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    _program: Program<'_>,
  ) {
    let is_js_like =
      matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx);

    for report_kind in context
      .all_comments()
      .filter_map(|comment| check_comment(comment, is_js_like))
    {
      context.add_diagnostic_with_hint(
        report_kind.range(),
        CODE,
        report_kind.as_message(),
        report_kind.as_hint(),
      );
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_invalid_triple_slash_reference.md")
  }
}

#[derive(Debug, Eq, PartialEq)]
enum ReportKind {
  /// In JavaScript files, the directives other than `types`, `path` and `lib` are not allowed. This variant
  /// represents such invalid directives in JavaScript.
  InvalidDirectiveInJs(SourceRange),

  /// Represents an unsupported or badly-formed directive
  InvalidDirective(SourceRange),
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

  fn range(&self) -> SourceRange {
    use ReportKind::*;
    match *self {
      InvalidDirectiveInJs(range) => range,
      InvalidDirective(range) => range,
    }
  }
}

// These regexes should be consistent with how Deno resolves modules.
// https://github.com/denoland/deno/blob/76e2edc7e1868d7768e259aacbb9a991e1afc462/cli/module_graph.rs
static TRIPLE_SLASH_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"(?i)^/\s*<reference\s.*?/>"#).unwrap());
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
fn check_comment(comment: &Comment, is_js_like: bool) -> Option<ReportKind> {
  if comment.kind == CommentKind::Block {
    return None;
  }
  if !TRIPLE_SLASH_REFERENCE_RE.is_match(&comment.text) {
    return None;
  }

  if is_js_like {
    // In JavaScript, only the `lib`, `no-default-lib`, `path` and `types` directives are allowed
    if is_types_ref(&comment.text)
      || is_lib_ref(&comment.text)
      || is_path_ref(&comment.text)
      || is_no_default_lib_ref(&comment.text)
    {
      None
    } else {
      Some(ReportKind::InvalidDirectiveInJs(comment.range()))
    }
  } else if is_path_ref(&comment.text)
    || is_types_ref(&comment.text)
    || is_lib_ref(&comment.text)
    || is_no_default_lib_ref(&comment.text)
  {
    None
  } else {
    Some(ReportKind::InvalidDirective(comment.range()))
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
  use deno_ast::StartSourcePos;
  use deno_ast::SourceRange;

  use super::*;

  const DUMMY_RANGE: SourceRange = SourceRange::new(StartSourcePos::START_SOURCE_POS.into(), StartSourcePos::START_SOURCE_POS.into());

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

  fn line(text: &str) -> Comment {
    Comment {
      kind: CommentKind::Line,
      span: DUMMY_RANGE.into(),
      text: text.to_string(),
    }
  }
  fn block(text: &str) -> Comment {
    Comment {
      kind: CommentKind::Block,
      span: DUMMY_RANGE.into(),
      text: text.to_string(),
    }
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
    for valid_comment in &valid_comments {
      assert!(check_comment(valid_comment, true).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference foo="./mod.d.ts" />"#),
      line(r#"/<reference bar />"#),
    ];
    for invalid_comment in &invalid_comments {
      let report_kind = check_comment(invalid_comment, true).unwrap();
      assert_eq!(report_kind, ReportKind::InvalidDirectiveInJs(DUMMY_RANGE))
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
    for valid_comment in &valid_comments {
      assert!(check_comment(valid_comment, false).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference foo="./mod.d.ts" />"#),
      line(r#"/<reference bar />"#),
    ];
    for invalid_comment in &invalid_comments {
      let report_kind = check_comment(invalid_comment, false).unwrap();
      assert_eq!(report_kind, ReportKind::InvalidDirective(DUMMY_RANGE))
    }
  }

  #[test]
  fn triple_slash_reference_valid() {
    // JavaScript
    assert_lint_ok! {
      NoInvalidTripleSlashReference,
      filename: "foo.js",
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
      filename: "foo.ts",
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
      let r = ReportKind::InvalidDirectiveInJs(DUMMY_RANGE);
      (r.as_message(), r.as_hint())
    };
    let (invalid_directive_msg, invalid_directive_hint) = {
      let r = ReportKind::InvalidDirective(DUMMY_RANGE);
      (r.as_message(), r.as_hint())
    };

    // JavaScript
    assert_lint_err! {
      NoInvalidTripleSlashReference,
      filename: "foo.js",
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
      filename: "foo.ts",
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
