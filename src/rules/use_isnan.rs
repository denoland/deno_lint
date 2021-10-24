// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view::{BinExpr, BinaryOp, Expr, Ident, SwitchStmt};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct UseIsNaN;

const CODE: &str = "use-isnan";

#[derive(Display)]
enum UseIsNaNMessage {
  #[display(fmt = "Use the isNaN function to compare with NaN")]
  Comparison,

  #[display(
    fmt = "'switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch"
  )]
  SwitchUnmatched,

  #[display(
    fmt = "'case NaN' can never match. Use Number.isNaN before the switch"
  )]
  CaseUnmatched,
}

impl LintRule for UseIsNaN {
  fn new() -> Arc<Self> {
    Arc::new(UseIsNaN)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    UseIsNaNHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/use_isnan.md")
  }
}

struct UseIsNaNHandler;

fn is_nan_identifier(ident: &Ident) -> bool {
  *ident.sym() == *"NaN"
}

impl Handler for UseIsNaNHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, ctx: &mut Context) {
    if bin_expr.op() == BinaryOp::EqEq
      || bin_expr.op() == BinaryOp::NotEq
      || bin_expr.op() == BinaryOp::EqEqEq
      || bin_expr.op() == BinaryOp::NotEqEq
      || bin_expr.op() == BinaryOp::Lt
      || bin_expr.op() == BinaryOp::LtEq
      || bin_expr.op() == BinaryOp::Gt
      || bin_expr.op() == BinaryOp::GtEq
    {
      if let Expr::Ident(ident) = bin_expr.left {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(
            bin_expr.span(),
            CODE,
            UseIsNaNMessage::Comparison,
          );
        }
      }
      if let Expr::Ident(ident) = bin_expr.right {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(
            bin_expr.span(),
            CODE,
            UseIsNaNMessage::Comparison,
          );
        }
      }
    }
  }

  fn switch_stmt(&mut self, switch_stmt: &SwitchStmt, ctx: &mut Context) {
    if let Expr::Ident(ident) = switch_stmt.discriminant {
      if is_nan_identifier(ident) {
        ctx.add_diagnostic(
          switch_stmt.span(),
          CODE,
          UseIsNaNMessage::SwitchUnmatched,
        );
      }
    }

    for case in &switch_stmt.cases {
      if let Some(Expr::Ident(ident)) = &case.test {
        if is_nan_identifier(ident) {
          ctx.add_diagnostic(case.span(), CODE, UseIsNaNMessage::CaseUnmatched);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn use_isnan_valid() {
    assert_lint_ok! {
      UseIsNaN,
      "var x = NaN;",
      "isNaN(NaN) === true;",
      "isNaN(123) !== true;",
      "Number.isNaN(NaN) === true;",
      "Number.isNaN(123) !== true;",
      "foo(NaN + 1);",
      "foo(1 + NaN);",
      "foo(NaN - 1)",
      "foo(1 - NaN)",
      "foo(NaN * 2)",
      "foo(2 * NaN)",
      "foo(NaN / 2)",
      "foo(2 / NaN)",
      "var x; if (x = NaN) { }",
      "var x = Number.NaN;",
      "isNaN(Number.NaN) === true;",
      "Number.isNaN(Number.NaN) === true;",
      "foo(Number.NaN + 1);",
      "foo(1 + Number.NaN);",
      "foo(Number.NaN - 1)",
      "foo(1 - Number.NaN)",
      "foo(Number.NaN * 2)",
      "foo(2 * Number.NaN)",
      "foo(Number.NaN / 2)",
      "foo(2 / Number.NaN)",
      "var x; if (x = Number.NaN) { }",
      "x === Number[NaN];",
    };
  }

  #[test]
  fn use_isnan_invalid() {
    assert_lint_err! {
      UseIsNaN,
      "42 === NaN": [
      {
        col: 0,
        message: UseIsNaNMessage::Comparison,
      }],
      r#"
switch (NaN) {
  case NaN:
    break;
  default:
    break;
}
        "#: [
      {
        line: 2,
        col: 0,
        message: UseIsNaNMessage::SwitchUnmatched,
      },
      {
        line: 3,
        col: 2,
        message: UseIsNaNMessage::CaseUnmatched,
      }],
    }
  }
}
