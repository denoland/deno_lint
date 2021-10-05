// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use ast_view::NodeTrait;
// use swc_common::Spanned;
use regex::Regex;
use swc_common::Spanned;

pub struct BanArrayLengthMinusOne;

const CODE: &str = "ban-array-length-minus-one";
const MESSAGE: &str = "arr[arr.length - 1] is deprecated.";
const HINT: &str = "Please consider using arr.at(-1) instead of arr[arr.length - 1]";

impl LintRule for BanArrayLengthMinusOne {
  fn new() -> Box<Self> {
    Box::new(BanArrayLengthMinusOne)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
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
    program: Program<'_>,
  ) {
      BanArrayLengthMinusOneHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_array_length_minus_one.md")
  }
}

struct BanArrayLengthMinusOneHandler;

impl Handler for BanArrayLengthMinusOneHandler {
  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    let obj = member_expr.obj;
    let prop = member_expr.prop;
    let mut regex_string:String = obj.text().to_owned();
    let property_string = ".length *- *1$";
    regex_string.push_str(property_string);
    let re = Regex::new(regex_string.as_str()).unwrap();

    if re.is_match(prop.text()) {
      ctx.add_diagnostic_with_hint(member_expr.prop.span(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_array_length_minus_one_valid() {
    assert_lint_ok! {
      BanArrayLengthMinusOne,
            r#"
const _x = fruits[fruits.length-2];
      "#
    }
  }

  #[test]
  fn ban_array_length_minus_one_invalid() {
    assert_lint_err! {
      BanArrayLengthMinusOne,
      MESSAGE,
      HINT,
            r#"
const _x = fruits[fruits.length-1];
      "#: [{ line: 2, col: 18 }],
            r#"
const _x = fruits[fruits.length - 1];
      "#: [{ line: 2, col: 18 }]
    }
  }
}
