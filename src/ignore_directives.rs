// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::diagnostic::{LintDiagnostic, Position};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::BytePos;
use swc_common::SourceMap;
use swc_common::Span;

static IGNORE_COMMENT_CODE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r",\s*|\s").unwrap());

#[derive(Clone, Debug, PartialEq)]
pub struct IgnoreDirective {
  pub position: Position,
  pub span: Span,
  pub codes: Vec<String>,
  pub used_codes: HashMap<String, bool>,
  pub is_global: bool,
}

impl IgnoreDirective {
  /// Check if `IgnoreDirective` supresses given `diagnostic` and if so
  /// mark the directive as used
  pub fn maybe_ignore_diagnostic(
    &mut self,
    diagnostic: &LintDiagnostic,
  ) -> bool {
    // `is_global` means that diagnostic is ignored in whole file.
    if self.is_global {
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

pub fn parse_ignore_directives(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  leading_comments: &HashMap<BytePos, Vec<Comment>>,
  trailing_comments: &HashMap<BytePos, Vec<Comment>>,
) -> Vec<IgnoreDirective> {
  let mut ignore_directives = vec![];

  leading_comments.values().for_each(|comments| {
    for comment in comments {
      if let Some(ignore) = parse_ignore_comment(
        &ignore_diagnostic_directive,
        source_map,
        comment,
        false,
      ) {
        ignore_directives.push(ignore);
      }
    }
  });

  trailing_comments.values().for_each(|comments| {
    for comment in comments {
      if let Some(ignore) = parse_ignore_comment(
        &ignore_diagnostic_directive,
        source_map,
        comment,
        false,
      ) {
        ignore_directives.push(ignore);
      }
    }
  });

  ignore_directives
    .sort_by(|a, b| a.position.line.partial_cmp(&b.position.line).unwrap());
  ignore_directives
}

pub fn parse_ignore_comment(
  ignore_diagnostic_directive: &str,
  source_map: &SourceMap,
  comment: &Comment,
  is_global: bool,
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
        is_global,
      });
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast_parser;
  use crate::ast_parser::AstParser;
  use std::rc::Rc;

  #[test]
  fn test_parse_ignore_comments() {
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
    let ast_parser = AstParser::new();
    let (_program, comments) = ast_parser
      .parse_program(
        "test.ts",
        ast_parser::get_default_ts_config(),
        &source_code,
      )
      .expect("Failed to parse");
    let (leading, trailing) = comments.take_all();
    let leading_coms = Rc::try_unwrap(leading)
      .expect("Failed to get leading comments")
      .into_inner();
    let trailing_coms = Rc::try_unwrap(trailing)
      .expect("Failed to get trailing comments")
      .into_inner();
    let leading = leading_coms.into_iter().collect();
    let trailing = trailing_coms.into_iter().collect();
    let directives = parse_ignore_directives(
      "deno-lint-ignore",
      &ast_parser.source_map,
      &leading,
      &trailing,
    );

    assert_eq!(directives.len(), 4);
    let d = &directives[0];
    assert_eq!(
      d.position,
      Position {
        line: 2,
        col: 0,
        byte_pos: 1
      }
    );
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
    let d = &directives[1];
    assert_eq!(
      d.position,
      Position {
        line: 8,
        col: 0,
        byte_pos: 146
      }
    );
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
    let d = &directives[2];
    assert_eq!(
      d.position,
      Position {
        line: 11,
        col: 0,
        byte_pos: 229
      }
    );
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
    let d = &directives[3];
    assert_eq!(
      d.position,
      Position {
        line: 17,
        col: 3,
        byte_pos: 388
      }
    );
    assert_eq!(d.codes, vec!["ban-types"]);
  }
}
