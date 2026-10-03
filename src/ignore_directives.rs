use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use deno_ast::SourceTextInfoProvider;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::view as ast_view;
use deno_ast::view::NodeTrait;
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
  /// For a directive that immediately precedes a `{ ... }` block, the line the
  /// block ends on (0-indexed, inclusive). The directive then suppresses
  /// diagnostics anywhere inside the block, not just on the following line.
  /// `None` for ordinary next-line directives.
  block_end_line: Option<usize>,
  _marker: std::marker::PhantomData<T>,
}

impl<T: DirectiveKind> IgnoreDirective<T> {
  pub fn range(&self) -> SourceRange {
    self.range
  }

  /// The last line (0-indexed, inclusive) covered by this directive when it is
  /// block-scoped; see [`IgnoreDirective::block_end_line`] field docs.
  pub fn block_end_line(&self) -> Option<usize> {
    self.block_end_line
  }

  fn set_block_end_line(&mut self, line: usize) {
    self.block_end_line = Some(line);
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
  let mut directives: HashMap<usize, LineIgnoreDirective> = program
    .comment_container()
    .all_comments()
    .filter_map(|comment| {
      parse_ignore_comment(ignore_diagnostic_directive, comment, true).map(
        |directive| {
          // Key by the line the comment *ends* on so that the directive
          // applies to the following line. This matters for block comments
          // (`/* ... */`) that may span multiple lines; for line comments
          // the start and end line are the same.
          (
            program.text_info().line_index(directive.range().end),
            directive,
          )
        },
      )
    })
    .collect();

  // When a directive immediately precedes a `{ ... }` block, extend its
  // coverage to the whole block. Only worth walking the AST if there are
  // directives to extend.
  if !directives.is_empty() {
    extend_block_ignore_directives(program, &mut directives);
  }

  directives
}

/// For every `{ ... }` block whose immediately preceding line is a
/// `deno-lint-ignore` directive, records the block's end line on that directive
/// so it suppresses diagnostics anywhere inside the block (see
/// https://github.com/denoland/deno_lint/issues/476).
///
/// Only bare/explicit blocks are covered: the directive must be a leading
/// comment of a `BlockStmt`. A directive placed before a function, class or
/// other statement attaches to that node rather than to a `BlockStmt`, so it
/// keeps the ordinary next-line behavior and does not silently swallow deeply
/// nested diagnostics.
fn extend_block_ignore_directives(
  program: ast_view::Program,
  directives: &mut HashMap<usize, LineIgnoreDirective>,
) {
  let text_info = program.text_info();
  let comments = program.comment_container();

  let mut stack = vec![program.as_node()];
  while let Some(node) = stack.pop() {
    if let ast_view::Node::BlockStmt(block) = node {
      let block_range = block.range();
      let block_start_line = text_info.line_index(block_range.start);
      let block_end_line = text_info.line_index(block_range.end);
      for comment in comments.leading_comments(block_range.start) {
        let comment_end_line = text_info.line_index(comment.range().end);
        // The directive must sit on the line directly above the block's `{`.
        if comment_end_line + 1 == block_start_line {
          if let Some(directive) = directives.get_mut(&comment_end_line) {
            directive.set_block_end_line(block_end_line);
          }
        }
      }
    }
    stack.extend(node.children());
  }
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
      module.body.first().map(SourceRanged::range),
    ),
    ast_view::Program::Script(script) => (
      script.shebang().is_some(),
      script.body.first().map(SourceRanged::range),
    ),
  };

  let comments = program.comment_container();
  let mut initial_comments = match (has_shebang, first_item_range) {
    (false, _) => comments.leading_comments(program.start()),
    (true, Some(range)) => comments.leading_comments(range.start),
    (true, None) => comments.trailing_comments(program.end()),
  };
  initial_comments.find_map(|comment| {
    parse_ignore_comment(ignore_global_directive, comment, false)
  })
}

fn parse_ignore_comment<T: DirectiveKind>(
  ignore_diagnostic_directive: &str,
  comment: &Comment,
  allow_block_comment: bool,
) -> Option<IgnoreDirective<T>> {
  // Line ignore directives may also be written as block comments, since
  // those are the only form available inside JSX children, e.g.
  // `{/* deno-lint-ignore react-no-danger */}`. File-level directives must
  // still be line comments.
  if comment.kind != CommentKind::Line
    && !(allow_block_comment && comment.kind == CommentKind::Block)
  {
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
        range: comment.range(),
        codes,
        block_end_line: None,
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

  #[test]
  fn test_block_scoped_directive_coverage() {
    // A directive directly above a bare `{ ... }` block records the block's
    // end line, so it covers the whole block.
    let source_code =
      "// deno-lint-ignore no-explicit-any\n{\n  let a: any;\n  let b: any;\n}";
    test_util::parse_and_then(source_code, |program| {
      let directives =
        parse_line_ignore_directives("deno-lint-ignore", program);
      let d = directives.get(&0).unwrap();
      // The closing `}` is on line 4 (0-indexed).
      assert_eq!(d.block_end_line(), Some(4));
    });

    // A directive above a function attaches to the function, not its body
    // block, so no block coverage is recorded ("blocks only").
    let source_code =
      "// deno-lint-ignore no-explicit-any\nfunction foo(): any {\n  let a: any;\n}";
    test_util::parse_and_then(source_code, |program| {
      let directives =
        parse_line_ignore_directives("deno-lint-ignore", program);
      let d = directives.get(&0).unwrap();
      assert_eq!(d.block_end_line(), None);
    });

    // An ordinary next-line directive (no following block) has no coverage.
    let source_code = "// deno-lint-ignore no-explicit-any\nconst a: any = 1;";
    test_util::parse_and_then(source_code, |program| {
      let directives =
        parse_line_ignore_directives("deno-lint-ignore", program);
      let d = directives.get(&0).unwrap();
      assert_eq!(d.block_end_line(), None);
    });
  }

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
  fn test_parse_block_comment_line_ignore_directives() {
    // Block comments are the only comment form available inside JSX
    // children, so they must work as line ignore directives. See
    // https://github.com/denoland/deno_lint/issues/1452
    let source_code = r#"
/* deno-lint-ignore no-explicit-any */
const a: any = 1;

/* deno-lint-ignore
   no-empty */
function foo() {}
"#;

    test_util::parse_and_then(source_code, |program| {
      let line_directives =
        parse_line_ignore_directives("deno-lint-ignore", program);

      assert_eq!(line_directives.len(), 2);
      // Single-line block comment on line 1 (0-indexed) suppresses line 2.
      let d = line_directives.get(&1).unwrap();
      assert_eq!(d.codes, code_map(["no-explicit-any"]));
      // Multi-line block comment is keyed by the line it *ends* on (5),
      // so it suppresses the following line.
      let d = line_directives.get(&5).unwrap();
      assert_eq!(d.codes, code_map(["no-empty"]));
    });
  }

  #[test]
  fn test_block_comment_not_a_file_directive() {
    // Block comments must not act as file-level ignore directives.
    test_util::parse_and_then("/* deno-lint-ignore-file */", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program);
      assert!(file_directive.is_none());
    });
  }

  #[test]
  fn test_parse_global_ignore_directives() {
    test_util::parse_and_then("// deno-lint-ignore-file", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program).unwrap();

      assert!(file_directive.codes.is_empty());
    });

    test_util::parse_and_then(
      "// deno-lint-ignore-file -- reason for ignoring",
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();

        assert!(file_directive.codes.is_empty());
      },
    );

    test_util::parse_and_then("// deno-lint-ignore-file foo", |program| {
      let file_directive =
        parse_file_ignore_directives("deno-lint-ignore-file", program).unwrap();

      assert_eq!(file_directive.codes, code_map(["foo"]));
    });

    test_util::parse_and_then(
      "// deno-lint-ignore-file foo -- reason for ignoring",
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();

        assert_eq!(file_directive.codes, code_map(["foo"]));
      },
    );

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
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file -- reason for ignoring",
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

    test_util::parse_and_then(
      "#!/usr/bin/env -S deno run\n// deno-lint-ignore-file -- reason for ignoring\nconst a = 42;",
      |program| {
        let file_directive =
          parse_file_ignore_directives("deno-lint-ignore-file", program)
            .unwrap();
        assert!(file_directive.codes.is_empty());
      },
    );
  }
}
