// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::oxc::ast::ast::Comment;
use deno_ast::oxc::ast::ast::CommentKind;
use deno_ast::oxc::span::Span;
use deno_ast::ParsedSource;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

pub type LineIgnoreDirective = IgnoreDirective<Line>;
pub type FileIgnoreDirective = IgnoreDirective<File>;

pub enum Line {}
pub enum File {}
pub trait DirectiveKind {}
impl DirectiveKind for Line {}
impl DirectiveKind for File {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoreDirective<T: DirectiveKind> {
  range: Span,
  codes: HashMap<String, CodeStatus>,
  _marker: std::marker::PhantomData<T>,
}

impl<T: DirectiveKind> IgnoreDirective<T> {
  pub fn range(&self) -> Span {
    self.range
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
  parsed_source: &ParsedSource,
) -> HashMap<usize, LineIgnoreDirective> {
  let source_text = parsed_source.text();
  let text_info = parsed_source.text_info_lazy();
  parsed_source
    .comments()
    .iter()
    .filter_map(|comment| {
      parse_ignore_comment(ignore_diagnostic_directive, comment, source_text)
        .map(|directive| {
          (
            text_info.line_index(directive.range().start as usize),
            directive,
          )
        })
    })
    .collect()
}

pub fn parse_file_ignore_directives(
  ignore_global_directive: &str,
  parsed_source: &ParsedSource,
) -> Option<FileIgnoreDirective> {
  let source_text = parsed_source.text();
  parsed_source
    .get_leading_comments()
    .find_map(|comment| {
      parse_ignore_comment(ignore_global_directive, comment, source_text)
    })
}

fn parse_ignore_comment<T: DirectiveKind>(
  ignore_diagnostic_directive: &str,
  comment: &Comment,
  source_text: &str,
) -> Option<IgnoreDirective<T>> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let content_span = comment.content_span();
  let comment_text =
    &source_text[content_span.start as usize..content_span.end as usize];
  let comment_text = comment_text.trim();

  if let Some(prefix) = comment_text.split_whitespace().next() {
    if prefix == ignore_diagnostic_directive {
      let comment_text = comment_text
        .strip_prefix(ignore_diagnostic_directive)
        .unwrap();

      static IGNORE_COMMENT_REASON_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\s*--.*").unwrap());

      // remove ignore reason
      let comment_text_without_reason =
        IGNORE_COMMENT_REASON_RE.replace_all(comment_text, "");

      static IGNORE_COMMENT_CODE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r",\s*|\s").unwrap());

      let comment_text =
        IGNORE_COMMENT_CODE_RE.replace_all(&comment_text_without_reason, ",");
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

      return Some(IgnoreDirective::<T> {
        range: comment.span,
        codes,
        _marker: std::marker::PhantomData,
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
Object | undefined {}

// deno-lint-ignore no-explicit-any no-empty no-debugger -- reason for ignoring
function foo(): any {}
  "#;

    test_util::parse_and_then(source_code, |parsed_source| {
      let line_directives = parse_line_ignore_directives(
        "deno-lint-ignore",
        parsed_source,
      );

      assert_eq!(line_directives.len(), 5);
      let d = line_directives.get(&1).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&7).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&10).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
      let d = line_directives.get(&16).unwrap();
      assert_eq!(d.codes, code_map(["ban-types"]));
      let d = line_directives.get(&19).unwrap();
      assert_eq!(
        d.codes,
        code_map(["no-explicit-any", "no-empty", "no-debugger"])
      );
    });
  }

  #[test]
  fn test_parse_global_ignore_directives() {
    test_util::parse_and_then("// deno-lint-ignore-file", |parsed_source| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
          .unwrap();

      assert!(file_directive.codes.is_empty());
    });

    test_util::parse_and_then(
      "// deno-lint-ignore-file -- reason for ignoring",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();

        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo -- reason for ignoring",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo bar",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo", "bar"]));
      },
    );

    test_util::parse_and_then(
      r#"
// deno-lint-ignore-file foo
// deno-lint-ignore-file bar
"#,
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      r#"
const x = 42;
// deno-lint-ignore-file foo
"#,
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source);

        assert!(file_directive.is_none());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file -- reason for ignoring",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file\nconst a = 42;",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file -- reason for ignoring\nconst a = 42;",
      |parsed_source| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", parsed_source)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );
  }
}
