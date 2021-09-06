// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{Expr, NewExpr, ParenExpr};
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::VisitAll;
use deno_ast::swc::visit::VisitAllWith;

#[derive(Debug)]
pub struct NoAsyncPromiseExecutor;

const CODE: &str = "no-async-promise-executor";
const MESSAGE: &str = "Async promise executors are not allowed";
const HINT: &str =
  "Remove `async` from executor function and adjust promise code as needed";

impl LintRule for NoAsyncPromiseExecutor {
  fn new() -> Box<Self> {
    Box::new(NoAsyncPromiseExecutor)
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
    let mut visitor = NoAsyncPromiseExecutorVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_async_promise_executor.md")
  }
}

struct NoAsyncPromiseExecutorVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoAsyncPromiseExecutorVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

fn is_async_function(expr: &Expr) -> bool {
  match expr {
    Expr::Fn(fn_expr) => fn_expr.function.is_async,
    Expr::Arrow(arrow_expr) => arrow_expr.is_async,
    Expr::Paren(ParenExpr { ref expr, .. }) => is_async_function(&**expr),
    _ => false,
  }
}

impl<'c, 'view> VisitAll for NoAsyncPromiseExecutorVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.as_ref();
      if name != "Promise" {
        return;
      }

      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          if is_async_function(&*first_arg.expr) {
            self.context.add_diagnostic_with_hint(
              new_expr.span,
              CODE,
              MESSAGE,
              HINT,
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_async_promise_executor_valid() {
    assert_lint_ok! {
      NoAsyncPromiseExecutor,
      "new Promise(function(resolve, reject) {});",
      "new Promise((resolve, reject) => {});",
      "new Promise((resolve, reject) => {}, async function unrelated() {})",
      "new Foo(async (resolve, reject) => {})",
      "new class { foo() { new Promise(function(resolve, reject) {}); } }",
    };
  }

  #[test]
  fn no_async_promise_executor_invalid() {
    assert_lint_err! {
      NoAsyncPromiseExecutor,
      "new Promise(async function(resolve, reject) {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(async function foo(resolve, reject) {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(async (resolve, reject) => {});": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Promise(((((async () => {})))));": [{ col: 0, message: MESSAGE, hint: HINT }],
      // nested
      r#"
const a = new class {
  foo() {
    let b = new Promise(async function(resolve, reject) {});
  }
}
      "#: [{ line: 4, col: 12, message: MESSAGE, hint: HINT }],
    }
  }
}
