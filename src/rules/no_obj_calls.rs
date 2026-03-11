// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  CallExpression, Expression, NewExpression, Program,
};
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoObjCalls;

const CODE: &str = "no-obj-calls";

fn get_message(callee_name: &str) -> String {
  format!("`{}` call as function is not allowed", callee_name)
}

impl LintRule for NoObjCalls {
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
    let mut handler = NoObjCallsHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoObjCallsHandler;

fn check_callee(
  ident: &deno_ast::oxc::ast::ast::IdentifierReference,
  span: Span,
  ctx: &mut Context,
) {
  let name = ident.name.as_str();
  if !matches!(name, "Math" | "JSON" | "Reflect" | "Atomics") {
    return;
  }
  // Check if the identifier resolves to a local binding via OXC scoping.
  if let Some(ref_id) = ident.reference_id.get() {
    let reference = ctx.scoping().get_reference(ref_id);
    if reference.symbol_id().is_some() {
      return; // Shadowed by a local binding
    }
  }
  ctx.add_diagnostic(span, "no-obj-calls", get_message(name));
}

impl Handler<'_> for NoObjCallsHandler {
  fn call_expression(
    &mut self,
    call_expr: &CallExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &call_expr.callee {
      check_callee(ident, call_expr.span, ctx);
    }
  }

  fn new_expression(
    &mut self,
    new_expr: &NewExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      check_callee(ident, new_expr.span, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_obj_calls_valid() {
    assert_lint_ok! {
      NoObjCalls,
      "Math.PI * 2 * 3;",
      r#"JSON.parse("{}");"#,
      r#"Reflect.get({ x: 1, y: 2 }, "x");"#,
      "Atomics.load(foo, 0);",
      r#"
function f(Math: () => void) {
  Math();
}
      "#,
      r#"
function f(JSON: () => void) {
  JSON();
}
      "#,
      r#"
function f(Reflect: () => void) {
  Reflect();
}
      "#,
      r#"
function f(Atomics: () => void) {
  Atomics();
}
      "#,
    };
  }

  #[test]
  fn no_obj_calls_invalid() {
    assert_lint_err! {
      NoObjCalls,
      "Math();": [{col: 0, message: get_message("Math")}],
      "new Math();": [{col: 0, message: get_message("Math")}],
      "JSON();": [{col: 0, message: get_message("JSON")}],
      "new JSON();": [{col: 0, message: get_message("JSON")}],
      "Reflect();": [{col: 0, message: get_message("Reflect")}],
      "new Reflect();": [{col: 0, message: get_message("Reflect")}],
      "Atomics();": [{col: 0, message: get_message("Atomics")}],
      "new Atomics();": [{col: 0, message: get_message("Atomics")}],
      r#"
function f(Math: () => void) { Math(); }
const m = Math();
      "#: [
        {
          col: 10,
          line: 3,
          message: get_message("Math"),
        },
      ],
    }
  }
}
