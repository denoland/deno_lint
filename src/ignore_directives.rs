// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::diagnostic::{LintDiagnostic, Position};
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
  codes: Vec<String>,
  used_codes: HashMap<String, bool>,
  kind: DirectiveKind,
}

impl IgnoreDirective {
  /// If the directive has no codes specified, it means all the rules should be
  /// ignored.
  pub fn ignore_all(&self) -> bool {
    self.codes.is_empty()
  }
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

  pub fn codes(&self) -> &[String] {
    &self.codes
  }

  pub fn used_codes(&self) -> &HashMap<String, bool> {
    &self.used_codes
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

    let mut should_ignore = false;
    for code in self.codes.iter() {
      if code == &diagnostic.code {
        should_ignore = true;
        *self.used_codes.get_mut(code).unwrap() = true;
      }
    }

    should_ignore
  }
}

pub fn parse_line_ignore_directives<'view>(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  comments: impl Iterator<Item = &'view Comment>,
) -> Vec<IgnoreDirective> {
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

pub fn parse_global_ignore_directives<'view>(
  ignore_global_directive: &str,
  source_map: &SourceMap,
  mut comments: impl Iterator<Item = &'view Comment>,
) -> Option<IgnoreDirective> {
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
        .filter(|code| !code.is_empty())
        .map(|code| String::from(code.trim()))
        .collect::<Vec<String>>();

      let location = source_map.lookup_char_pos(comment.span.lo());
      let position = Position::new(comment.span.lo(), location);
      let mut used_codes = HashMap::new();
      codes.iter().for_each(|code| {
        used_codes.insert(code.to_string(), false);
      });

      return Some(IgnoreDirective {
        position,
        span: comment.span,
        codes,
        used_codes,
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
  use dprint_swc_ecma_ast_view::{RootNode, Spanned};

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
      let line_directives = parse_line_ignore_directives(
        "deno-lint-ignore",
        &source_map,
        program.comments().unwrap().all_comments(),
      );

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
      assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
      let d = &line_directives[1];
      assert_eq!(
        d.position,
        Position {
          line: 8,
          col: 0,
          byte_pos: 146
        }
      );
      assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
      let d = &line_directives[2];
      assert_eq!(
        d.position,
        Position {
          line: 11,
          col: 0,
          byte_pos: 229
        }
      );
      assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
      let d = &line_directives[3];
      assert_eq!(
        d.position,
        Position {
          line: 17,
          col: 3,
          byte_pos: 388
        }
      );
      assert_eq!(d.codes, vec!["ban-types"]);
    });
  }

  #[test]
  fn test_parse_global_ignore_directives() {
    let source_code = "// deno-lint-ignore-file";

    test_util::parse_and_then(source_code, |program, source_map| {
      let global_directive = parse_global_ignore_directives(
        "deno-lint-ignore-file",
        &source_map,
        program.comments().unwrap().leading_comments(program.span().lo()),
      ).unwrap();

      assert_eq!(
        global_directive.position,
        Position {
          line: 1,
          col: 0,
          byte_pos: 0
        }
      );
      assert!(global_directive.codes.is_empty());
    });
  }
}
