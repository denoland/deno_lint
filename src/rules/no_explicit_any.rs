// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct NoExplicitAny;

const CODE: &str = "no-explicit-any";
const MESSAGE: &str = "`any` type is not allowed";
const HINT: &str = "Use a specific type other than `any`";

impl LintRule for NoExplicitAny {
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
    let mut handler = NoExplicitAnyHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoExplicitAnyHandler;

impl Handler<'_> for NoExplicitAnyHandler {
  fn ts_any_keyword(
    &mut self,
    ts_any_keyword: &TSAnyKeyword,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_hint(ts_any_keyword.span, CODE, MESSAGE, HINT);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_explicit_any_valid() {
    assert_lint_ok! {
      NoExplicitAny,
      r#"
class Foo {
  static _extensions: {
    // deno-lint-ignore no-explicit-any
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#,
      r#"
type RequireWrapper = (
  // deno-lint-ignore no-explicit-any
  exports: any,
  // deno-lint-ignore no-explicit-any
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#,
    };
  }

  #[test]
  fn no_explicit_any_invalid() {
    assert_lint_err! {
      NoExplicitAny,
      "function foo(): any { return undefined; }": [{ col: 16, message: MESSAGE, hint: HINT }],
      "function bar(): Promise<any> { return undefined; }": [{ col: 24, message: MESSAGE, hint: HINT }],
      "const a: any = {};": [{ col: 9, message: MESSAGE, hint: HINT }],
      r#"
class Foo {
  static _extensions: {
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#: [{ line: 4, col: 57, message: MESSAGE, hint: HINT }],
      r#"
type RequireWrapper = (
  exports: any,
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#: [{ line: 3, col: 11, message: MESSAGE, hint: HINT }, { line: 4, col: 11, message: MESSAGE, hint: HINT }],
    }
  }
}
