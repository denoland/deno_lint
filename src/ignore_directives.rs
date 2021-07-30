// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use dprint_swc_ecma_ast_view::{self as ast_view, RootNode, Spanned};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::SourceMap;
use swc_common::Span;

pub type LineIgnoreDirective = IgnoreDirective<Line>;
pub type FileIgnoreDirective = IgnoreDirective<File>;

pub enum Line {}
pub enum File {}
pub trait DirectiveKind {}
impl DirectiveKind for Line {}
impl DirectiveKind for File {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoreDirective<T: DirectiveKind> {
  span: Span,
  codes: HashMap<String, CodeStatus>,
  _marker: std::marker::PhantomData<T>,
}

impl<T: DirectiveKind> IgnoreDirective<T> {
  pub fn span(&self) -> Span {
    self.span
  }

  /// If the directive has no codes specified, it means all the rules should be
  /// ignored.
  pub fn ignore_all(&self) -> bool {
    self.codes.is_empty()
  }

  pub fn codes(&self) -> &HashMap<String, CodeStatus> {
    &self.codes
  }

  pub fn has_code(&self, code: &str) -> bool {
    self.codes.contains_key(code)
  }

  pub fn check_used(&mut self, diagnostic_code: &str) -> bool {
    if let Some(status) = self.codes.get_mut(diagnostic_code) {
      status.mark_as_used();
      true
    } else {
      false
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeStatus {
  pub used: bool,
}

impl CodeStatus {
  fn mark_as_used(&mut self) {
    self.used = true;
  }
}

pub fn parse_line_ignore_directives(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  program: ast_view::Program,
) -> HashMap<usize, LineIgnoreDirective> {
  program
    .comments()
    .unwrap()
    .all_comments()
    .filter_map(|comment| {
      parse_ignore_comment(ignore_diagnostic_directive, source_map, comment)
    })
    .collect()
}

pub fn parse_file_ignore_directives(
  ignore_global_directive: &str,
  source_map: &SourceMap,
  program: ast_view::Program,
) -> Option<FileIgnoreDirective> {
  program
    .comments()
    .unwrap()
    .leading_comments(program.span().lo())
    .find_map(|comment| {
      parse_ignore_comment(ignore_global_directive, source_map, comment)
        .map(|(_pos, dir)| dir)
    })
}

fn parse_ignore_comment<T: DirectiveKind>(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  comment: &Comment,
) -> Option<(usize, IgnoreDirective<T>)> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let comment_text = comment.text.trim();

  if let Some(prefix) = comment_text.split_whitespace().next() {
    if prefix == ignore_diagnostic_directive {
      let comment_text = comment_text
        .strip_prefix(ignore_diagnostic_directive)
        .unwrap();

      static IGNORE_COMMENT_CODE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r",\s*|\s").unwrap());

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

      return Some((
        location.line,
        IgnoreDirective::<T> {
          span: comment.span,
          codes,
          _marker: std::marker::PhantomData,
        },
      ));
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
      let d = line_directives.get(&2).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&8).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&11).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&17).unwrap();
      assert_eq!(d.codes, code_map(["ban-types"]));
    });
  }

  #[test]
  fn test_parse_global_ignore_directives() {
    test_util::parse_and_then(
      "// deno-lint-ignore-file",
      |program, source_map| {
        let file_directive = parse_file_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo",
      |program, source_map| {
        let file_directive = parse_file_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo bar",
      |program, source_map| {
        let file_directive = parse_file_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo", "bar"]));
      },
    );

    test_util::parse_and_then(
      r#"
// deno-lint-ignore-file foo
// deno-lint-ignore-file bar
"#,
      |program, source_map| {
        let file_directive = parse_file_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        )
        .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      r#"
const x = 42;
// deno-lint-ignore-file foo
"#,
      |program, source_map| {
        let file_directive = parse_file_ignore_directives(
          "deno-lint-ignore-file",
          &source_map,
          program,
        );

        assert!(file_directive.is_none());
      },
    );
  }
}
