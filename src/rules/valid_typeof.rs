// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecma_ast::BinaryOp::{EqEq, EqEqEq, NotEq, NotEqEq};
use swc_ecma_ast::Expr::{Lit, Unary};
use swc_ecma_ast::Lit::Str;
use swc_ecma_ast::UnaryOp::TypeOf;
use swc_ecma_ast::{BinExpr, Module};
use swc_ecma_visit::{Node, Visit};
pub struct ValidTypeof;

impl LintRule for ValidTypeof {
  fn new() -> Box<Self> {
    Box::new(ValidTypeof)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = ValidTypeofVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct ValidTypeofVisitor {
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
      (Unary(unary), operand @ _) | (operand @ _, Unary(unary))
        if unary.op == TypeOf =>
      {
        match operand {
          Lit(Str(str)) if !is_valid_typeof_string(&str.value.to_string()) => {
            self.context.add_diagnostic(
              str.span,
              "validTypeof",
              "Invalid typeof comparison value",
            );
          }
          _ => {}
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
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn it_passes_using_valid_strings() {
    test_lint(
      "valid_typeof",
      r#"
typeof foo === "string"
typeof bar == "undefined"
     "#,
      vec![ValidTypeof::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_not_using_strings() {
    test_lint(
      "valid_typeof",
      r#"
typeof foo === undefined
typeof bar == Object
     "#,
      vec![ValidTypeof::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_not_two_typeof_operations() {
    test_lint(
      "valid_typeof",
      r#"
typeof bar === typeof qux
     "#,
      vec![ValidTypeof::new()],
      json!([]),
    )
  }

  #[test]
  fn it_fails_using_invalid_strings() {
    test_lint(
      "valid_typeof",
      r#"
typeof foo === "strnig"
typeof foo == "undefimed"
typeof bar != "nunber"
typeof bar !== "fucntion"
     "#,
      vec![ValidTypeof::new()],
      json!([
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 2,
            "col": 15,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 3,
            "col": 14,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 4,
            "col": 14,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 5,
            "col": 15,
          }
        },
      ]),
    )
  }
}
