// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{NumericLiteral, Program};
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct NoOctal;

const CODE: &str = "no-octal";
const MESSAGE: &str = "Numeric literals beginning with `0` are not allowed";
const HINT: &str = "To express octal numbers, use `0o` as a prefix instead";

impl LintRule for NoOctal {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoOctalHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoOctalHandler;

impl Handler<'_> for NoOctalHandler {
  fn numeric_literal(
    &mut self,
    literal_num: &NumericLiteral,
    ctx: &mut Context,
  ) {
    static OCTAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^0[0-9]").unwrap());

    let Some(ref raw) = literal_num.raw else {
      return;
    };
    let raw_number = raw.as_str();

    if OCTAL.is_match(raw_number) {
      ctx.add_diagnostic_with_hint(literal_num.span, CODE, MESSAGE, HINT);
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
