// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::diagnostic::{LintDiagnostic, Position};
use dprint_swc_ecma_ast_view::{self as ast_view, RootNode, Spanned};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::SourceMap;
use swc_common::Span;

static IGNORE_COMMENT_CODE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r",\s*|\s").unwrap());

#[derive(Clone, Debug, PartialEq)]
pub struct IgnoreDirective {
  position: Position,
  span: Span,
  codes: HashMap<String, CodeStatus>,
  kind: DirectiveKind,
}

impl IgnoreDirective {
  /// If the directive has no codes specified, it means all the rules should be
  /// ignored.
  pub fn ignore_all(&self) -> bool {
    self.codes.is_empty()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeStatus {
  pub used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveKind {
  /// The directive has an effect on the whole file.
  Global,

  /// The directive has an effect on the next line.
  Line,
}

impl DirectiveKind {
  fn is_global(&self) -> bool {
    matches!(*self, DirectiveKind::Global)
  }

  // TODO(magurotuna) remove
  #[allow(unused)]
  fn is_line(&self) -> bool {
    matches!(*self, DirectiveKind::Line)
  }
}

impl IgnoreDirective {
  pub fn span(&self) -> Span {
    self.span
  }

  pub fn codes(&self) -> &HashMap<String, CodeStatus> {
    &self.codes
  }

  /// Check if `IgnoreDirective` supresses given `diagnostic` and if so
  /// mark the directive as used
  pub fn maybe_ignore_diagnostic(
    &mut self,
    diagnostic: &LintDiagnostic,
  ) -> bool {
    if self.kind.is_global() {
      // pass
    } else if self.position.line != diagnostic.range.start.line - 1 {
      return false;
    }

    if let Some(code_status) = self.codes.get_mut(&diagnostic.code) {
      code_status.used = true;
      true
    } else {
      false
    }
  }
}

pub fn parse_line_ignore_directives(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  program: ast_view::Program,
) -> Vec<IgnoreDirective> {
  let comments = program.comments().unwrap().all_comments();
  let mut ignore_directives: Vec<IgnoreDirective> = comments
    .filter_map(|comment| {
      parse_ignore_comment(
        &ignore_diagnostic_directive,
        source_map,
        comment,
        DirectiveKind::Line,
      )
    })
    .collect();
  ignore_directives.sort_by_key(|d| d.position.line);
  ignore_directives
}

pub fn parse_global_ignore_directives(
  ignore_global_directive: &str,
  source_map: &SourceMap,
  program: ast_view::Program,
) -> Option<IgnoreDirective> {
  let mut comments = program
    .comments()
    .unwrap()
    .leading_comments(program.span().lo());
  comments.find_map(|comment| {
    parse_ignore_comment(
      ignore_global_directive,
      source_map,
      comment,
      DirectiveKind::Global,
    )
  })
}

fn parse_ignore_comment(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  comment: &Comment,
  kind: DirectiveKind,
) -> Option<IgnoreDirective> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let comment_text = comment.text.trim();

  if let Some(prefix) = comment_text.split_whitespace().next() {
    if prefix == ignore_diagnostic_directive {
      let comment_text = comment_text
        .strip_prefix(ignore_diagnostic_directive)
        .unwrap();
      let comment_text = IGNORE_COMMENT_CODE_RE.replace_all(comment_text, ",");
      let codes = comment_text
        .split(',')
        .filter_map(|code| {
          if code.is_empty() {
            None
          } else {
            let code = code.trim().to_string();
            Some((code, CodeStatus::default()))
          }
        })
        .collect();

      let location = source_map.lookup_char_pos(comment.span.lo());
      let position = Position::new(comment.span.lo(), location);

      return Some(IgnoreDirective {
        position,
        span: comment.span,
        codes,
        kind,
      });
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util;

  fn code_map(
    codes: impl IntoIterator<Item = &'static str>,
  ) -> HashMap<String, CodeStatus> {
    codes
      .into_iter()
      .map(|code| (code.to_string(), CodeStatus::default()))
      .collect()
  }

  #[test]
  fn test_parse_line_ignore_comments() {
    let source_code = r#"
// deno-lint-ignore no-explicit-any no-empty no-debugger
function foo(): any {}

// not-deno-lint-ignore no-explicit-any
function foo(): any {}

// deno-lint-ignore no-explicit-any, no-empty, no-debugger
function foo(): any {}

// deno-lint-ignore no-explicit-any,no-empty,no-debugger
function foo(): any {}

export function deepAssign(
target: Record<string, any>,
...sources: any[]
): // deno-lint-ignore ban-types
object | undefined {}
  "#;

    test_util::parse_and_then(source_code, |program, source_map| {
      let line_directives =
        parse_line_ignore_directives("deno-lint-ignore", &source_map, program);

      assert_eq!(line_directives.len(), 4);
      let d = &line_directives[0];
      assert_eq!(
        d.position,
        Position {
          line: 2,
          col: 0,
          byte_pos: 1
        }
      );
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = &line_directives[1];
      assert_eq!(
        d.position,
        Position {
          line: 8,
          col: 0,
          byte_pos: 146
        }
      );
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = &line_directives[2];
      assert_eq!(
        d.position,
        Position {
          line: 11,
          col: 0,
          byte_pos: 229
        }
      );
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = &line_directives[3];
      assert_eq!(
        d.position,
        Position {
          line: 17,
          col: 3,
          byte_pos: 388
        }
      );
      assert_eq!(d.codes, code_map(["ban-types"]));
    });
  }

  #[test]
  fn test_parse_global_ignore_directives() {
    test_util::parse_and_then(
      "// deno-lint-ignore-file",
      |program, source_map| {
        let global_directive = parse_global_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(
          global_directive.position,
          Position {
            line: 1,
            col: 0,
            byte_pos: 0
          }
        );
        assert!(global_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo",
      |program, source_map| {
        let global_directive = parse_global_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(
          global_directive.position,
          Position {
            line: 1,
            col: 0,
            byte_pos: 0
          }
        );
        assert_eq!(global_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo bar",
      |program, source_map| {
        let global_directive = parse_global_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(
          global_directive.position,
          Position {
            line: 1,
            col: 0,
            byte_pos: 0
          }
        );
        assert_eq!(global_directive.codes, code_map(["foo", "bar"]));
      },
    );

    test_util::parse_and_then(
      r#"
// deno-lint-ignore-file foo
// deno-lint-ignore-file bar
"#,
      |program, source_map| {
        let global_directive = parse_global_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(
          global_directive.position,
          Position {
            line: 2,
            col: 0,
            byte_pos: 1
          }
        );
        assert_eq!(global_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      r#"
const x = 42;
// deno-lint-ignore-file foo
"#,
      |program, source_map| {
        let global_directive = parse_global_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        );

        assert!(global_directive.is_none());
      },
    );
  }
}
