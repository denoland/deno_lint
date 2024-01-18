use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use deno_ast::SourceTextInfoProvider;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::view as ast_view;
use deno_ast::RootNode;
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
  range: SourceRange,
  codes: HashMap<String, CodeStatus>,
  _marker: std::marker::PhantomData<T>,
}

impl<T: DirectiveKind> IgnoreDirective<T> {
  pub fn range(&self) -> SourceRange {
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
  program: ast_view::Program,
) -> HashMap<usize, LineIgnoreDirective> {
  program
    .comment_container()
    .all_comments()
    .filter_map(|comment| {
      parse_ignore_comment(ignore_diagnostic_directive, comment).map(
        |directive| {
          (
            program.text_info().line_index(directive.range().start),
            directive,
          )
        },
      )
    })
    .collect()
}

pub fn parse_file_ignore_directives(
  ignore_global_directive: &str,
  program: ast_view::Program,
) -> Option<FileIgnoreDirective> {
  // We want to get a file's leading comments, even if they come after a
  // shebang. There are three cases:
  // 1. No shebang. The file's leading comments are the program's leading
  //    comments.
  // 2. Shebang, and the program has statements or declarations. The file's
  //    leading comments are really the first statment/declaration's leading
  //    comments.
  // 3. Shebang, and the program is empty. The file's leading comments are the
  //    program's trailing comments.
  let (has_shebang, first_item_range) = match program {
    ast_view::Program::Module(module) => (
      module.shebang().is_some(),
      module.body.get(0).map(SourceRanged::range),
    ),
    ast_view::Program::Script(script) => (
      script.shebang().is_some(),
      script.body.get(0).map(SourceRanged::range),
    ),
  };

  let comments = program.comment_container();
  let mut initial_comments = match (has_shebang, first_item_range) {
    (false, _) => comments.leading_comments(program.start()),
    (true, Some(range)) => comments.leading_comments(range.start),
    (true, None) => comments.trailing_comments(program.end()),
  };
  initial_comments
    .find_map(|comment| parse_ignore_comment(ignore_global_directive, comment))
}

fn parse_ignore_comment<T: DirectiveKind>(
  ignore_diagnostic_directive: &str,
  comment: &Comment,
) -> Option<IgnoreDirective<T>> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let comment_text = comment.text.trim();

  if let Some(prefix) = comment_text.split_whitespace().next() {
    if prefix == ignore_diagnostic_directive {
      let comment_text = comment_text
        .strip_prefix(ignore_diagnostic_directive)
        .unwrap();

      static IGNORE_COMMENT_REASON_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\s*--.*").unwrap());

      // remove ignore reason
      let comment_text_without_reason = IGNORE_COMMENT_REASON_RE.replace_all(comment_text, "");

      static IGNORE_COMMENT_CODE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r",\s*|\s").unwrap());

      let comment_text = IGNORE_COMMENT_CODE_RE.replace_all(&comment_text_without_reason, ",");
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
        range: comment.range(),
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
object | undefined {}

// deno-lint-ignore no-explicit-any no-empty no-debugger -- reason for ignoring
function foo(): any {}
  "#;

    test_util::parse_and_then(source_code, |program| {
      let line_directives =
        parse_line_ignore_directives("deno-lint-ignore", program);

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
    test_util::parse_and_then("// deno-lint-ignore-file", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program).unwrap();

      assert!(file_directive.codes.is_empty());
    });

    test_util::parse_and_then("// deno-lint-ignore-file foo", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program).unwrap();

      assert_eq!(file_directive.codes, code_map(["foo"]));
    });

    test_util::parse_and_then("// deno-lint-ignore-file foo bar", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program).unwrap();

      assert_eq!(file_directive.codes, code_map(["foo", "bar"]));
    });

    test_util::parse_and_then(
      r#"
// deno-lint-ignore-file foo
// deno-lint-ignore-file bar
"#,
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

    test_util::parse_and_then(
      r#"
const x = 42;
// deno-lint-ignore-file foo
"#,
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program);

        assert!(file_directive.is_none());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file",
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file\nconst a = 42;",
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );
  }
}
