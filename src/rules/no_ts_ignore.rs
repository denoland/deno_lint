use super::{Context, LintRule};
use crate::tags::Tags;
use crate::Program;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::{SourceRange, SourceRangedForSpanned};
use once_cell::sync::Lazy;
use regex::Regex;

const CODE: &str = "no-ts-ignore";

const MESSAGE: &str = "@ts-ignore is not allowed.";

const HINT: &str = "Remove @ts-ignore and check your type declaration.";

#[derive(Debug)]
pub struct NoTsIgnore;

impl NoTsIgnore {
  fn report(&self, range: SourceRange, context: &mut Context) {
    context.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
  }
}

impl LintRule for NoTsIgnore {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    _program: Program,
  ) {
    static IGNORE_REGEX: Lazy<Regex> =
      Lazy::new(|| Regex::new(r"@ts-ignore(?::\s*[^\n]*|[^\n]*)?$").unwrap());

    for comment in context.all_comments() {
      if comment.kind != CommentKind::Line {
        continue;
      }

      if IGNORE_REGEX.is_match(&comment.text) {
        self.report(comment.range(), context);
      }
    }
  }
}

mod tests {
  use super::*;

  #[test]
  fn no_ts_ignore_valid() {
    assert_lint_ok! {
      NoTsIgnore,
      r#"/* @ts-ignore */"#,
      r#"/** @ts-ignore */"#,
      r#"/*
// @ts-ignore in a block
*/
"#,
    };
  }

  #[test]
  fn no_ts_ignore_invalid() {
    assert_lint_err! {
      NoTsIgnore,
      r#"// @ts-ignore"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"if (false) {
// @ts-ignore: Unreachable code error
console.log('hello');
}"#: [
      {
        line: 2,
        message: MESSAGE,
        hint: HINT,
      }
    ],
      r#"// @ts-ignore"# : [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
      ],
      r#"/// @ts-ignore"# : [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
      ],
      r#"//@ts-ignore"# : [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
      ],
      r#"// @ts-ignore    "# : [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
      ],
    }
  }
}
