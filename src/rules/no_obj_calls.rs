// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, Ident, NewExpr};
use deno_ast::{SourceRange, SourceRanged};

#[derive(Debug)]
pub struct NoObjCalls;

const CODE: &str = "no-obj-calls";

fn get_message(callee_name: &str) -> String {
  format!("`{}` call as function is not allowed", callee_name)
}

impl LintRule for NoObjCalls {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoObjCallsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_obj_calls.md")
  }
}

struct NoObjCallsHandler;

fn check_callee(callee: &Ident, range: SourceRange, ctx: &mut Context) {
  if matches!(
    callee.sym().as_ref(),
    "Math" | "JSON" | "Reflect" | "Atomics"
  ) && ctx.scope().var(&callee.to_id()).is_none()
  {
    ctx.add_diagnostic(
      range,
      "no-obj-calls",
      get_message(callee.sym().as_ref()),
    );
  }
}

impl Handler for NoObjCallsHandler {
  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let Callee::Expr(Expr::Ident(ident)) = call_expr.callee {
      check_callee(ident, call_expr.range(), ctx);
    }
  }

  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    if let Expr::Ident(ident) = new_expr.callee {
      check_callee(ident, new_expr.range(), ctx);
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
