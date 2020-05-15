use super::{Context, LintRule};
use swc_ecma_ast::{Module, BinaryOp, BinExpr, Expr};
use swc_ecma_ast::BinaryOp::*;
use swc_ecma_ast::Expr::{Lit, Unary};
use swc_ecma_ast::Lit::Num;
use swc_ecma_ast::UnaryOp::Minus;
use swc_ecma_visit::{Node, Visit};

pub struct NoCompareNegZero;

impl LintRule for NoCompareNegZero {
  fn new() -> Box<Self> {
    Box::new(NoCompareNegZero)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoCompareNegZeroVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoCompareNegZeroVisitor {
  context: Context,
}

impl NoCompareNegZeroVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoCompareNegZeroVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if is_comp_expr(&bin_expr.op) {
      println!("{:?} is compare expr", bin_expr.op);
      println!("LeftExpr -> {:?}", bin_expr.left);
      println!("RightExpr -> {:?}", bin_expr.right);
      let left_neg_zero = is_neg_zero(&*bin_expr.left);
      let right_neg_zero = is_neg_zero(&*bin_expr.right);
      println!("Left: {:?}. Right: {:?}", left_neg_zero, right_neg_zero);

      if left_neg_zero || right_neg_zero {
        self.context.add_diagnostic(
          bin_expr.span,
          "noCompareNegZero",
          format!("Do not compare against -0").as_str(),
        );
      }
    }
  }
}

fn is_comp_expr(binary_op: &BinaryOp) -> bool {
  match binary_op {
    EqEq
    | NotEq
    | EqEqEq
    | NotEqEq
    | Lt
    | LtEq
    | Gt
    | GtEq => true,
    _ => false,
  }
}

fn is_neg_zero(expr: &Expr) -> bool {
  match expr {
    Lit(lit) => match lit {
      Num(number) => {
        println!("Number: {:?}", number.value);
        number.value == 0.0 && number.value.is_sign_negative()
      },
      _ => false,
    },
    Unary(unary) => {
      if let Minus = &unary.op {
        if let Lit(lit) = &*unary.arg {
          if let Num(number) = lit {
            return number.value == 0.0;
          }
        }
      }
      false
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn it_fails_using_triple_eq_with_neg_zero() {
    test_lint(
      "no_compare_neg_zero",
      r#"
if (x === -0) {
}
     "#,
      vec![NoCompareNegZero::new()],
      json!([{
        "code": "noCompareNegZero",
        "message": "Do not compare against -0",
        "location": {
          "filename": "no_compare_neg_zero",
          "line": 2,
          "col": 4,
        }
      }]),
    )
  }
}
