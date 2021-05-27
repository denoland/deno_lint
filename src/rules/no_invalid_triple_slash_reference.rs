// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::comments::{Comment, CommentKind};
use swc_common::Span;

pub struct NoInvalidTripleSlashReference;

const CODE: &str = "no-invalid-triple-slash-reference";

#[derive(Display)]
enum NoInvalidTripleSlashReferenceMessage {
  #[display(
    fmt = "triple-slash references with `path` have no effect in Deno"
  )]
  Path,
}

#[derive(Display)]
enum NoInvalidTripleSlashReferenceHint {
  #[display(
    fmt = r#"Replace `path` with `types`, like `/// <reference types="{}" />`"#,
    _0
  )]
  Replace(String),
}

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
    for (span, path) in context.all_comments().filter_map(check_comment) {
      context.add_diagnostic_with_hint(
        span,
        CODE,
        NoInvalidTripleSlashReferenceMessage::Path,
        NoInvalidTripleSlashReferenceHint::Replace(path),
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

/// Returns `Some` if the comment should be reported.
fn check_comment(comment: &Comment) -> Option<(Span, String)> {
  if comment.kind == CommentKind::Block {
    return None;
  }

  static INVALID_TSR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^/\s*<reference\s*path\s*=\s*["|'](?P<path>.*)["|']\s*/>"#)
      .unwrap()
  });

  INVALID_TSR_REGEX
    .captures_iter(&comment.text)
    .next()
    .map(|cap| (comment.span, cap["path"].to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_check_comment() {
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

    let valid_comments = [
      line(r#"/ <reference types="./mod.d.ts" />"#),
      line(r#"/<reference types="./mod.d.ts" />"#),
      line(r#"/      <reference types="./mod.d.ts"     />           foo bar "#),
      line(r#"/ foo <reference path="./mod.d.ts" />"#),
      line(r#"<reference path="./mod.d.ts" />"#),
      block(r#"<reference path="./mod.d.ts" />"#),
    ];
    for valid_comment in &valid_comments {
      assert!(check_comment(valid_comment).is_none());
    }

    let invalid_comments = [
      line(r#"/ <reference path="./mod.d.ts" />"#),
      line(r#"/<reference path="./mod.d.ts" />"#),
      line(r#"/      <reference path="./mod.d.ts"     />           foo bar "#),
    ];
    for invalid_comment in &invalid_comments {
      let (span, text) = check_comment(invalid_comment).unwrap();
      assert_eq!(span, swc_common::DUMMY_SP);
      assert_eq!(text, "./mod.d.ts");
    }
  }

  #[test]
  fn triple_slash_reference_valid() {
    assert_lint_ok! {
      NoInvalidTripleSlashReference,
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
        /// <reference path="foo" />
        */
        import * as foo from 'foo';"#,
    };
  }

  #[test]
  fn triple_slash_reference_invalid() {
    assert_lint_err! {
      NoInvalidTripleSlashReference,
      r#"/// <reference path="foo" />"#: [
        {
          line: 1,
          col: 0,
          message: NoInvalidTripleSlashReferenceMessage::Path,
          hint: variant!(NoInvalidTripleSlashReferenceHint, Replace, "foo"),
        },
      ],
      r#"/// <reference path="foo" />hello"#: [
        {
          line: 1,
          col: 0,
          message: NoInvalidTripleSlashReferenceMessage::Path,
          hint: variant!(NoInvalidTripleSlashReferenceHint, Replace, "foo"),
        },
      ],
      r#"///<reference path="foo"/>"#: [
        {
          line: 1,
          col: 0,
          message: NoInvalidTripleSlashReferenceMessage::Path,
          hint: variant!(NoInvalidTripleSlashReferenceHint, Replace, "foo"),
        },
      ],
      r#"///        <reference      path       =   "foo" />"#: [
        {
          line: 1,
          col: 0,
          message: NoInvalidTripleSlashReferenceMessage::Path,
          hint: variant!(NoInvalidTripleSlashReferenceHint, Replace, "foo"),
        },
      ],
    };
  }
}
