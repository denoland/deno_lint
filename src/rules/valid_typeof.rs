// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_common::Spanned;
use crate::swc_ecma_ast::BinaryOp::{EqEq, EqEqEq, NotEq, NotEqEq};
use crate::swc_ecma_ast::Expr::{Lit, Unary};
use crate::swc_ecma_ast::Lit::Str;
use crate::swc_ecma_ast::UnaryOp::TypeOf;
use crate::swc_ecma_ast::{BinExpr, Module};
use swc_ecma_visit::{Node, Visit};

pub struct ValidTypeof;

impl LintRule for ValidTypeof {
  fn new() -> Box<Self> {
    Box::new(ValidTypeof)
  }

  fn code(&self) -> &'static str {
    "valid-typeof"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = ValidTypeofVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct ValidTypeofVisitor {
  context: Context,
}

impl ValidTypeofVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for ValidTypeofVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
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
            if !is_valid_typeof_string(&str.value) {
              self.context.add_diagnostic(
                str.span,
                "valid-typeof",
                "Invalid typeof comparison value",
              );
            }
          }
          _ => {
            self.context.add_diagnostic(
              operand.span(),
              "valid-typeof",
              "Invalid typeof comparison value",
            );
          }
        }
      }
      _ => {}
    }
  }
}

fn is_valid_typeof_string(str: &str) -> bool {
  match str {
    "undefined" | "object" | "boolean" | "number" | "string" | "function"
    | "symbol" | "bigint" => true,
    _ => false,
  }
}

trait EqExpr {
  fn is_eq_expr(&self) -> bool;
}

impl EqExpr for BinExpr {
  fn is_eq_expr(&self) -> bool {
    match self.op {
      EqEq | NotEq | EqEqEq | NotEqEq => true,
      _ => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_using_valid_strings() {
    assert_lint_ok::<ValidTypeof>(
      r#"
typeof foo === "string"
typeof bar == "undefined"
      "#,
    );
  }

  #[test]
  fn it_passes_using_two_typeof_operations() {
    assert_lint_ok::<ValidTypeof>(r#"typeof bar === typeof qux"#);
  }

  #[test]
  fn it_fails_using_invalid_strings() {
    assert_lint_err::<ValidTypeof>(r#"typeof foo === "strnig""#, 15);
    assert_lint_err::<ValidTypeof>(r#"typeof foo == "undefimed""#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof bar != "nunber""#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof bar !== "fucntion""#, 15);
  }

  #[test]
  fn it_fails_not_using_strings() {
    assert_lint_err::<ValidTypeof>(r#"typeof foo === undefined"#, 15);
    assert_lint_err::<ValidTypeof>(r#"typeof bar == Object"#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof baz === anotherVariable"#, 15);
  }
}
