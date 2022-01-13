// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{CallExpr, Callee, Expr, ExprOrSpread, NewExpr};
use std::sync::Arc;

#[derive(Debug)]
pub struct NoArrayConstructor;

const CODE: &str = "no-array-constructor";
const MESSAGE: &str = "Array Constructor is not allowed";
const HINT: &str = "Use array literal notation (e.g. []) or single argument specifying array size only (e.g. new Array(5)";

impl LintRule for NoArrayConstructor {
  fn new() -> Arc<Self> {
    Arc::new(NoArrayConstructor)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    _context: &mut Context<'view>,
    _program: ProgramRef<'view>,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoArrayConstructorHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_array_constructor.md")
  }
}

fn check_args(args: Vec<&ExprOrSpread>, span: Span, context: &mut Context) {
  if args.len() != 1 {
    context.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
  }
}

struct NoArrayConstructorHandler;

impl Handler for NoArrayConstructorHandler {
  fn new_expr(&mut self, new_expr: &NewExpr, context: &mut Context) {
    if let Expr::Ident(ident) = &new_expr.callee {
      let name = ident.inner.as_ref();
      if name != "Array" {
        return;
      }
      if new_expr.type_args.is_some() {
        return;
      }
      match &new_expr.args {
        Some(args) => {
          check_args(args.to_vec(), new_expr.span(), context);
        }
        None => check_args(vec![], new_expr.span(), context),
      };
    }
  }

  fn call_expr(&mut self, call_expr: &CallExpr, context: &mut Context) {
    if let Callee::Expr(Expr::Ident(ident)) = &call_expr.callee {
      let name = ident.inner.as_ref();
      if name != "Array" {
        return;
      }
      if call_expr.type_args.is_some() {
        return;
      }

      check_args((&*call_expr.args).to_vec(), call_expr.span(), context);
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
