// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, AwaitExpression, ForOfStatement, Function, Program,
};

#[derive(Debug)]
pub struct NoTopLevelAwait;

const CODE: &str = "no-top-level-await";
const MESSAGE: &str = "Top level await is not allowed";

impl LintRule for NoTopLevelAwait {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoTopLevelAwaitHandler { fn_depth: 0 };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoTopLevelAwaitHandler {
  fn_depth: u32,
}

impl Handler<'_> for NoTopLevelAwaitHandler {
  fn function(&mut self, _n: &Function, _ctx: &mut Context) {
    self.fn_depth += 1;
  }

  fn function_exit(&mut self, _n: &Function, _ctx: &mut Context) {
    self.fn_depth -= 1;
  }

  fn arrow_function_expression(
    &mut self,
    _n: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self.fn_depth += 1;
  }

  fn arrow_function_expression_exit(
    &mut self,
    _n: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self.fn_depth -= 1;
  }

  fn await_expression(
    &mut self,
    await_expr: &AwaitExpression,
    ctx: &mut Context,
  ) {
    if self.fn_depth == 0 {
      ctx.add_diagnostic(await_expr.span, CODE, MESSAGE);
    }
  }

  fn for_of_statement(
    &mut self,
    for_of_stmt: &ForOfStatement,
    ctx: &mut Context,
  ) {
    if for_of_stmt.r#await && self.fn_depth == 0 {
      ctx.add_diagnostic(for_of_stmt.span, CODE, MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_top_level_await_valid() {
    assert_lint_ok! {
      NoTopLevelAwait,
      r#"async function foo() { await bar(); }"#,
      r#"const foo = async function () { await bar()};"#,
      r#"const foo = () => { await bar()};"#,
      r#"async function foo() { for await (item of items){}}"#,
      r#"async function foo() { await bar(); }"#,
      r#"class Foo {
        async foo() { await task(); }
        private async bar(){ await task(); }
      }"#,
      r#"const foo = { bar : async () => { await task()} }"#,
    };
  }

  #[test]
  fn no_top_level_await_invalid() {
    assert_lint_err! {
      NoTopLevelAwait,
      r#"await foo()"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"for await (item of items) {}"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
    };
  }
}
