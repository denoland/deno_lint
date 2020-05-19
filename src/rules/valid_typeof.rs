use super::{Context, LintRule};
use swc_ecma_ast::BinaryOp::*;
use swc_ecma_ast::Expr::{Lit, Unary};
use swc_ecma_ast::Lit::Str;
use swc_ecma_ast::UnaryOp::{Minus, TypeOf};
use swc_ecma_ast::{BinExpr, BinaryOp, Module, UnaryExpr};
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
    if !bin_expr.op.is_comparator() {
      return;
    }

    match (&*bin_expr.left, &*bin_expr.right) {
      (Unary(unary), Lit(Str(str))) | (Lit(Str(str)), Unary(unary))
        if unary.op == TypeOf =>
      {
        if !is_valid_typeof_string(&str.value.to_string()) {
          self.context.add_diagnostic(
            bin_expr.span,
            "validTypeof",
            "Invalid typeof comparison value",
          );
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

trait Comparator {
  fn is_comparator(&self) -> bool;
}

impl Comparator for BinaryOp {
  fn is_comparator(&self) -> bool {
    match self {
      EqEq | NotEq | EqEqEq | NotEqEq | Lt | LtEq | Gt | GtEq => true,
      _ => false,
    }
  }
}

trait TypeOfExpr {
  fn is_typeof_expr(&self) -> bool;
}

impl TypeOfExpr for UnaryExpr {
  fn is_typeof_expr(&self) -> bool {
    self.op == TypeOf
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
typeof foo === baz
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
            "col": 0,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 3,
            "col": 0,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 4,
            "col": 0,
          }
        },
        {
          "code": "validTypeof",
          "message": "Invalid typeof comparison value",
          "location": {
            "filename": "valid_typeof",
            "line": 5,
            "col": 0,
          }
        },
      ]),
    )
  }
}
