// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::BinExpr;
use deno_ast::swc::ast::BinaryOp::{EqEq, EqEqEq, NotEq, NotEqEq};
use deno_ast::swc::ast::Expr::{Ident, Lit, Tpl, Unary};
use deno_ast::swc::ast::Lit::Str;
use deno_ast::swc::ast::UnaryOp::TypeOf;
use deno_ast::swc::ecma_visit::{noop_visit_type, Visit};
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct ValidTypeof;

const CODE: &str = "valid-typeof";
const MESSAGE: &str = "Invalid typeof comparison value";

impl LintRule for ValidTypeof {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let program = program_ref(program);
    let mut visitor = ValidTypeofVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
  }

  fn priority(&self) -> u32 {
    0
  }
}

struct ValidTypeofVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> ValidTypeofVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl Visit for ValidTypeofVisitor<'_, '_> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr) {
    if !bin_expr.is_eq_expr() {
      return;
    }

    match (&*bin_expr.left, &*bin_expr.right) {
      (Unary(unary), operand) | (operand, Unary(unary))
        if unary.op == TypeOf =>
      {
        match operand {
          Unary(unary) if unary.op == TypeOf => {}
          Lit(Str(str)) => {
            if !is_valid_typeof_string(&str.value.to_string_lossy()) {
              self.context.add_diagnostic(str.range(), CODE, MESSAGE);
            }
          }
          Tpl(tpl) => {
            if tpl
              .string_repr()
              .is_some_and(|s| !is_valid_typeof_string(&s))
            {
              self.context.add_diagnostic(tpl.range(), CODE, MESSAGE);
            }
          }
          // The bare `undefined` keyword is almost always a mistake here (it
          // should be the string `"undefined"`), so keep flagging it.
          Ident(ident) if ident.sym == *"undefined" => {
            self.context.add_diagnostic(ident.range(), CODE, MESSAGE);
          }
          // A non-string literal (number, boolean, `null`, ...) can never be a
          // valid `typeof` result.
          Lit(lit) => {
            self.context.add_diagnostic(lit.range(), CODE, MESSAGE);
          }
          // Comparing against a variable or other expression is allowed;
          // TypeScript already checks the value.
          // https://github.com/denoland/deno_lint/issues/1375
          _ => {}
        }
      }
      _ => {}
    }
  }
}

fn is_valid_typeof_string(str: &str) -> bool {
  matches!(
    str,
    "undefined"
      | "object"
      | "boolean"
      | "number"
      | "string"
      | "function"
      | "symbol"
      | "bigint"
  )
}

trait EqExpr {
  fn is_eq_expr(&self) -> bool;
}

impl EqExpr for BinExpr {
  fn is_eq_expr(&self) -> bool {
    matches!(self.op, EqEq | NotEq | EqEqEq | NotEqEq)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_typeof_valid() {
    assert_lint_ok! {
      ValidTypeof,
      r#"typeof foo === "undefined""#,
      r#"typeof foo === "object""#,
      r#"typeof foo === "boolean""#,
      r#"typeof foo === "number""#,
      r#"typeof foo === "string""#,
      r#"typeof foo === "function""#,
      r#"typeof foo === "symbol""#,
      r#"typeof foo === "bigint""#,

      r#"typeof foo == 'undefined'"#,
      r#"typeof foo == 'object'"#,
      r#"typeof foo == 'boolean'"#,
      r#"typeof foo == 'number'"#,
      r#"typeof foo == 'string'"#,
      r#"typeof foo == 'function'"#,
      r#"typeof foo == 'symbol'"#,
      r#"typeof foo == 'bigint'"#,

      // https://github.com/denoland/deno_lint/issues/741
      r#"typeof foo !== `undefined`"#,
      r#"typeof foo !== `object`"#,
      r#"typeof foo !== `boolean`"#,
      r#"typeof foo !== `number`"#,
      r#"typeof foo !== `string`"#,
      r#"typeof foo !== `function`"#,
      r#"typeof foo !== `symbol`"#,
      r#"typeof foo !== `bigint`"#,

      r#"typeof bar != typeof qux"#,

      // https://github.com/denoland/deno_lint/issues/1375
      // Comparing against a variable or other expression is allowed.
      r#"const type = "string"; typeof foo === type"#,
      r#"typeof bar == Object"#,
      r#"typeof baz === anotherVariable"#,
      r#"typeof foo === obj.type"#,
      r#"typeof foo === getExpected()"#,
    };
  }

  #[test]
  fn valid_typeof_invalid() {
    assert_lint_err! {
      ValidTypeof,
      r#"typeof foo === "strnig""#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo == "undefimed""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar != "nunber""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar !== "fucntion""#: [{
        col: 15,
        message: MESSAGE
      }],
      // The bare `undefined` keyword is still flagged (likely a mistake).
      r#"typeof foo === undefined"#: [{
        col: 15,
        message: MESSAGE
      }],
      // Non-string literals can never be a valid `typeof` result.
      r#"typeof foo === null"#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo === 5"#: [{
        col: 15,
        message: MESSAGE
      }],
    }
  }
}
