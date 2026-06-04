// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Argument, CallExpression, Expression, NewExpression, Program,
};
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoArrayConstructor;

const CODE: &str = "no-array-constructor";
const MESSAGE: &str = "Array Constructor is not allowed";
const HINT: &str = "Use array literal notation (e.g. []) or single argument specifying array size only (e.g. new Array(5)";

impl LintRule for NoArrayConstructor {
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
    let mut handler = NoArrayConstructorHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

fn check_args(args: &[Argument], span: Span, context: &mut Context) {
  if args.len() != 1 {
    context.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
  }
}

struct NoArrayConstructorHandler;

impl Handler<'_> for NoArrayConstructorHandler {
  fn new_expression(
    &mut self,
    new_expr: &NewExpression,
    context: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      if ident.name.as_str() != "Array" {
        return;
      }
      if new_expr.type_arguments.is_some() {
        return;
      }
      check_args(&new_expr.arguments, new_expr.span, context);
    }
  }

  fn call_expression(
    &mut self,
    call_expr: &CallExpression,
    context: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &call_expr.callee {
      if ident.name.as_str() != "Array" {
        return;
      }
      if call_expr.type_arguments.is_some() {
        return;
      }
      check_args(&call_expr.arguments, call_expr.span, context);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_array_constructor_valid() {
    assert_lint_ok! {
      NoArrayConstructor,
      "Array(x)",
      "Array(9)",
      "Array.foo()",
      "foo.Array()",
      "new Array(x)",
      "new Array(9)",
      "new foo.Array()",
      "new Array.foo",
      "new Array<Foo>(1, 2, 3);",
      "new Array<Foo>()",
      "Array<Foo>(1, 2, 3);",
      "Array<Foo>();",
    };
  }

  #[test]
  fn no_array_constructor_invalid() {
    assert_lint_err! {
      NoArrayConstructor,
      "new Array": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array()": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array(x, y)": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array(0, 1, 2)": [{ col: 0, message: MESSAGE, hint: HINT }],
      // nested
      r#"
const a = new class {
  foo() {
    let arr = new Array();
  }
}();
      "#: [{ line: 4, col: 14, message: MESSAGE, hint: HINT }],
      r#"
const a = (() => {
  let arr = new Array();
})();
      "#: [{ line: 3, col: 12, message: MESSAGE, hint: HINT }],
    }
  }
}
