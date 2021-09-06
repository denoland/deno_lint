// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{CallExpr, Expr, ExprOrSuper, Ident, NewExpr};
use deno_ast::swc::common::Span;
use deno_ast::swc::utils::ident::IdentLike;
use deno_ast::swc::visit::{noop_visit_type, Node, Visit};

#[derive(Debug)]
pub struct NoObjCalls;

const CODE: &str = "no-obj-calls";

fn get_message(callee_name: &str) -> String {
  format!("`{}` call as function is not allowed", callee_name)
}

impl LintRule for NoObjCalls {
  fn new() -> Box<Self> {
    Box::new(NoObjCalls)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoObjCallsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_obj_calls.md")
  }
}

struct NoObjCallsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoObjCallsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_callee(&mut self, callee: &Ident, span: Span) {
    if matches!(callee.sym.as_ref(), "Math" | "JSON" | "Reflect" | "Atomics")
      && self.context.scope().var(&callee.to_id()).is_none()
    {
      self.context.add_diagnostic(
        span,
        "no-obj-calls",
        get_message(callee.sym.as_ref()),
      );
    }
  }
}

impl<'c, 'view> Visit for NoObjCallsVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        self.check_callee(ident, call_expr.span);
      }
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      self.check_callee(ident, new_expr.span);
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
