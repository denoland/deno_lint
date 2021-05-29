// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::comments::{Comment, CommentKind};
use swc_common::Span;

pub struct NoInvalidTripleSlashReference;

const CODE: &str = "no-invalid-triple-slash-reference";

impl LintRule for NoInvalidTripleSlashReference {
  fn new() -> Box<Self> {
    Box::new(NoInvalidTripleSlashReference)
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
    _program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    let is_js_like = is_js_or_jsx(context.file_name());

    for report_kind in context
      .all_comments()
      .filter_map(|comment| check_comment(comment, is_js_like))
    {
      context.add_diagnostic_with_hint(
        report_kind.span(),
        CODE,
        report_kind.as_message(),
        report_kind.as_hint(),
      );
    }
  }

  fn docs(&self) -> &'static str {
    r#"Warns the usage of triple-slash references directive of `path`.

Deno supports the triple-slash reference `types` directive, which is useful for
telling the TypeScript compiler the location of a type definition file that
corresponds to a certain JavaScript file.

However, in the Deno manual of the previous versions (e.g. [v1.9.2]), there was
a wrong statement describing that one should use the `path` directive. Actually,
the `types` directive should be used. See [the latest manual] for more detail.

[v1.9.2]: https://deno.land/manual@v1.9.2/typescript/types#using-the-triple-slash-reference-directive
[the latest manual]: https://deno.land/manual/typescript/types#using-the-triple-slash-reference-directive

This lint rule detects such wrong usage of the `path` directive and suggests
replacing it with the `types` directive.

### Invalid:
```javascript
/// <reference path="./mod.d.ts" />

// ... the rest of the JavaScript ...
```

### Valid:
```javascript
/// <reference types="./mod.d.ts" />

// ... the rest of the JavaScript ...
```
"#
  }
}

// TODO(@magurotuna): use MediaType instead
// https://github.com/denoland/deno/blob/76e2edc7e1868d7768e259aacbb9a991e1afc462/cli/media_type.rs#L15-L26
fn is_js_or_jsx(filename: &str) -> bool {
  filename.ends_with(".js")
    || filename.ends_with(".mjs")
    || filename.ends_with(".cjs")
    || filename.ends_with(".jsx")
}

#[derive(Debug, Eq, PartialEq)]
enum ReportKind {
  /// In JavaScript files, the directives other than `types` are not allowed. This variant
  /// represents such invalid directives in JavaScript.
  NotTypesInJs(Span),

  /// Represents an unsupported or badly-formed directive
  InvalidDirective(Span),
}

impl ReportKind {
  fn as_message(&self) -> &'static str {
    use ReportKind::*;
    match *self {
      NotTypesInJs(_) => {
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
      NotTypesInJs(_) => {
        r#"In JavaScript only the `types` directive is allowed, like `/// <reference types="..." />`"#
      }
      InvalidDirective(_) => {
        r#"Correct format is `/// <reference xxx="some_value" />` where `xxx` is one of `types`, `path`, and `lib`"#
      }
    }
  }

  fn span(&self) -> Span {
    use ReportKind::*;
    match *self {
      NotTypesInJs(span) => span,
      InvalidDirective(span) => span,
    }
  }
}

// These four regexes should be consistent with how Deno resolves modules.
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

/// Returns `Some` if the comment should be reported.
fn check_comment(comment: &Comment, is_js_like: bool) -> Option<ReportKind> {
  if comment.kind == CommentKind::Block {
    return None;
  }
  if !TRIPLE_SLASH_REFERENCE_RE.is_match(&comment.text) {
    return None;
  }

  if is_js_like {
    // In JavaScript, only the `types` directives are allowed
    if is_types_ref(&comment.text) {
      None
    } else {
      Some(ReportKind::NotTypesInJs(comment.span))
    }
  } else {
    if is_path_ref(&comment.text)
      || is_types_ref(&comment.text)
      || is_lib_ref(&comment.text)
    {
      None
    } else {
      Some(ReportKind::InvalidDirective(comment.span))
    }
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

#[cfg(test)]
mod tests {
  use super::*;

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
  fn test_is_js_or_jsx() {
    let testcases = [
      ("foo.js", true),
      ("foo.jsx", true),
      ("foo.mjs", true),
      ("foo.cjs", true),
      ("foo.bar.js", true),
      ("foo.ts", false),
      ("foo.tsx", false),
    ];

    for (input, expected) in &testcases {
      assert_eq!(*expected, is_js_or_jsx(input));
    }
  }

  fn line(text: &str) -> Comment {
    Comment {
      kind: CommentKind::Line,
      span: swc_common::DUMMY_SP,
      text: text.to_string(),
    }
  }
  fn block(text: &str) -> Comment {
    Comment {
      kind: CommentKind::Block,
      span: swc_common::DUMMY_SP,
      text: text.to_string(),
    }
  }

  #[test]
  fn test_check_comment_js() {
    let valid_comments = [
      line(r#"/ <reference types="./mod.d.ts" />"#),
      line(r#"/<reference types="./mod.d.ts" />"#),
      line(r#"/      <reference types="./mod.d.ts"     />           foo bar "#),
      // normal comment because of inserted "foo"
      line(r#"/ foo <reference path="./mod.d.ts" />"#),
      // normal comment because of inserted "foo"
      line(r#"/ foo <reference lib="./mod.d.ts" />"#),
      // just double slash
      line(r#"<reference path="./mod.d.ts" />"#),
      // just double slash
      line(r#"<reference lib="./mod.d.ts" />"#),
      // block comment
      block(r#"<reference path="./mod.d.ts" />"#),
      // block comment
      block(r#"<reference lib="./mod.d.ts" />"#),
    ];
    for valid_comment in &valid_comments {
      assert!(check_comment(valid_comment, true).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference path="./mod.d.ts" />"#),
      line(r#"/ <reference lib="./mod.d.ts" />"#),
      line(r#"/<reference path="./mod.d.ts" />"#),
      line(r#"/      <reference path="./mod.d.ts"     />           foo bar "#),
      line(r#"/ <reference foo="./mod.d.ts" />"#),
      line(r#"/<reference bar />"#),
    ];
    for invalid_comment in &invalid_comments {
      let report_kind = check_comment(invalid_comment, true).unwrap();
      assert_eq!(report_kind, ReportKind::NotTypesInJs(swc_common::DUMMY_SP))
    }
  }

  #[test]
  fn test_check_comment_not_js() {
    let valid_comments = [
      line(r#"/ <reference types="./mod.d.ts" />"#),
      line(r#"/ <reference path="./mod.d.ts" />"#),
      line(r#"/ <reference lib="./mod.d.ts" />"#),
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
      assert_eq!(
        report_kind,
        ReportKind::InvalidDirective(swc_common::DUMMY_SP)
      )
    }
  }

  #[test]
  fn triple_slash_reference_valid() {
    // JavaScript
    assert_lint_ok! {
      NoInvalidTripleSlashReference,
      filename: "foo.js",
      r#"/// <reference types="./mod.d.ts" />"#,
      r#"// <reference path="path" />"#,
      r#"// <reference lib="lib" />"#,
      r#"/* <reference path="path" /> */"#,
      r#"/* <reference lib="lib" /> */"#,
      r#"/// hello <reference path="./mod.d.ts" />"#,
      r#"
        /*
        /// <reference path="foo" />
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
    };
  }

  #[test]
  fn triple_slash_reference_invalid() {
    let (not_types_in_js_msg, not_types_in_js_hint) = {
      let r = ReportKind::NotTypesInJs(swc_common::DUMMY_SP);
      (r.as_message(), r.as_hint())
    };
    let (invalid_directive_msg, invalid_directive_hint) = {
      let r = ReportKind::InvalidDirective(swc_common::DUMMY_SP);
      (r.as_message(), r.as_hint())
    };

    // JavaScript
    assert_lint_err! {
      NoInvalidTripleSlashReference,
      filename: "foo.js",
      r#"/// <reference path="foo" />"#: [
        {
          line: 1,
          col: 0,
          message: not_types_in_js_msg,
          hint: not_types_in_js_hint,
        },
      ],
      r#"/// <reference lib="foo" />"#: [
        {
          line: 1,
          col: 0,
          message: not_types_in_js_msg,
          hint: not_types_in_js_hint,
        },
      ],
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
