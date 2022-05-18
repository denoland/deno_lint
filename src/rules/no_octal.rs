// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::view::Number;
use deno_ast::SourceRanged;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoOctal;

const CODE: &str = "no-octal";
const MESSAGE: &str = "Numeric literals beginning with `0` are not allowed";
const HINT: &str = "To express octal numbers, use `0o` as a prefix instead";

impl LintRule for NoOctal {
  fn new() -> Arc<Self> {
    Arc::new(NoOctal)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoOctalHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_octal.md")
  }
}

struct NoOctalHandler;

impl Handler for NoOctalHandler {
  fn number(&mut self, literal_num: &Number, ctx: &mut Context) {
    static OCTAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^0[0-9]").unwrap());

    let raw_number = ctx.file_text_substring(&literal_num.range());

    if OCTAL.is_match(raw_number) {
      ctx.add_diagnostic_with_hint(literal_num.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_octal_valid() {
    assert_lint_ok! {
      NoOctal,
      "7",
      "\"07\"",
      "0x08",
      "-0.01",
    };
  }

  #[test]
  fn no_octal_invalid() {
    assert_lint_err! {
      NoOctal,
      "07": [{col: 0, message: MESSAGE, hint: HINT}],
      "let x = 7 + 07": [{col: 12, message: MESSAGE, hint: HINT}],

      // https://github.com/denoland/deno/issues/10954
      // Make sure it doesn't panic
      "020000000000000000000;": [{col: 0, message: MESSAGE, hint: HINT}],
    }
  }
}
