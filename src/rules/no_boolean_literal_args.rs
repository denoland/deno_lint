// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view::{Expr, ExprOrSpread, Lit};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoBooleanLiteralArgs;

const CODE: &str = "no-boolean-literal-args";

#[derive(Display)]
enum NoBooleanLiteralArgsMessage {
  #[display(fmt = "Not allowed to use true or false as an argument")]
  Unexpected,
}

#[derive(Display)]
enum NoBooleanLiteralArgsHint {
  #[display(fmt = "Create a boolean to pass to the function instead")]
  CreateBoolean,
}

impl LintRule for NoBooleanLiteralArgs {
  fn new() -> Arc<Self> {
    Arc::new(NoBooleanLiteralArgs)
  }

  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    "no-boolean-literal-args"
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let mut handler = NoBooleanLiteralArgsHandler::default();
    handler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_boolean_literal_args.md")
  }
}

#[derive(Default)]
struct NoBooleanLiteralArgsHandler;

impl NoBooleanLiteralArgsHandler {
  fn check_args<'a>(&'a mut self, args: &[&ExprOrSpread], ctx: &mut Context) {
    for arg in args {
      if let Expr::Lit(Lit::Bool(_)) = &arg.expr {
        ctx.add_diagnostic_with_hint(
          arg.expr.span(),
          CODE,
          NoBooleanLiteralArgsMessage::Unexpected,
          NoBooleanLiteralArgsHint::CreateBoolean,
        );
      }
    }
  }
}

impl Handler for NoBooleanLiteralArgsHandler {
  fn call_expr(&mut self, expr: &deno_ast::view::CallExpr, ctx: &mut Context) {
    self.check_args(&expr.args, ctx);
  }

  fn new_expr(&mut self, expr: &deno_ast::view::NewExpr, ctx: &mut Context) {
    if let Some(args) = &expr.args {
      self.check_args(args, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_boolean_literal_args_valid() {
    assert_lint_ok! {
      NoBooleanLiteralArgs,
      "let x = func(CONST_BOOL_1, 5, 3)",
      "let x = func(trueBool, 0)",
      "let x = func(y, 1, z)",
      "func(falseBool, CONST_BOOL_1)",
      "func(CONST_BOOL_1, func2(CONST_BOOL_2))",
    };
  }

  #[test]
  fn no_boolean_literal_args_invalid() {
    assert_lint_err! {
      NoBooleanLiteralArgs,
      "func(true, a, b)": [
        {
          col: 5,
          message: NoBooleanLiteralArgsMessage::Unexpected,
          hint: NoBooleanLiteralArgsHint::CreateBoolean,
        }
      ],
      "func(a, b, true)": [
        {
          col: 11,
          message: NoBooleanLiteralArgsMessage::Unexpected,
          hint: NoBooleanLiteralArgsHint::CreateBoolean,
        }
      ],
      "let x = func(true)": [
        {
          col: 13,
          message: NoBooleanLiteralArgsMessage::Unexpected,
          hint: NoBooleanLiteralArgsHint::CreateBoolean,
        }
      ],
      "let x = func(func2(false))": [
        {
          col: 19,
          message: NoBooleanLiteralArgsMessage::Unexpected,
          hint: NoBooleanLiteralArgsHint::CreateBoolean,
        }
      ],
      "let x = new MyObject(false)": [
        {
          col: 21,
          message: NoBooleanLiteralArgsMessage::Unexpected,
          hint: NoBooleanLiteralArgsHint::CreateBoolean,
        }
      ]
    };
  }
}
