// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::{BytePos, Span};

#[derive(Debug)]
pub struct PreferAscii;

const CODE: &str = "prefer-ascii";
const MESSAGE: &str = "Non-ASCII characters are not allowed";

fn hint(c: char) -> String {
  format!("`{}` is not an ASCII. Consider replacing it", c)
}

impl LintRule for PreferAscii {
  fn new() -> Box<Self> {
    Box::new(PreferAscii)
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
    _program: Program<'_>,
  ) {
    let mut not_asciis = Vec::new();

    let mut src_chars = context.source_file().text().char_indices().peekable();
    while let Some((i, c)) = src_chars.next() {
      if let Some(&(pi, _)) = src_chars.peek() {
        if (pi > i + 1) || !c.is_ascii() {
          let span = Span::new(
            BytePos(i as u32),
            BytePos(pi as u32),
            Default::default(),
          );
          not_asciis.push((c, span));
        }
      }
    }

    for (c, span) in not_asciis {
      context.add_diagnostic_with_hint(span, CODE, MESSAGE, hint(c));
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/prefer_ascii.md")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefer_ascii_valid() {
    assert_lint_ok! {
      PreferAscii,
      r#"const pi = Math.PI;"#,
      r#"const ninja = "ninja";"#,
      r#"
function hello(name: string) {
  console.log(`Hello, ${name}`);
}
      "#,
      r#"// "comments" are also checked"#,
      r#"/* "comments" are also checked */"#,
    };
  }

  #[test]
  fn prefer_ascii_invalid() {
    assert_lint_err! {
      PreferAscii,
      r#"const π = Math.PI;"#: [
        {
          line: 1,
          col: 6,
          message: MESSAGE,
          hint: hint('π'),
        },
      ],
      r#"const ninja = "🥷";"#: [
        {
          line: 1,
          col: 15,
          message: MESSAGE,
          hint: hint('🥷'),
        },
      ],
      r#"function こんにちは(名前: string) {}"#: [
        {
          line: 1,
          col: 9,
          message: MESSAGE,
          hint: hint('こ'),
        },
        {
          line: 1,
          col: 10,
          message: MESSAGE,
          hint: hint('ん'),
        },
        {
          line: 1,
          col: 11,
          message: MESSAGE,
          hint: hint('に'),
        },
        {
          line: 1,
          col: 12,
          message: MESSAGE,
          hint: hint('ち'),
        },
        {
          line: 1,
          col: 13,
          message: MESSAGE,
          hint: hint('は'),
        },
        {
          line: 1,
          col: 15,
          message: MESSAGE,
          hint: hint('名'),
        },
        {
          line: 1,
          col: 16,
          message: MESSAGE,
          hint: hint('前'),
        },
      ],
      r#"// “comments” are also checked"#: [
        {
          line: 1,
          col: 3,
          message: MESSAGE,
          hint: hint('“'),
        },
        {
          line: 1,
          col: 12,
          message: MESSAGE,
          hint: hint('”'),
        },
      ],
      r#"/* “comments” are also checked */"#: [
        {
          line: 1,
          col: 3,
          message: MESSAGE,
          hint: hint('“'),
        },
        {
          line: 1,
          col: 12,
          message: MESSAGE,
          hint: hint('”'),
        },
      ],
    };
  }
}
