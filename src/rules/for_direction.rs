// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::AssignExpr;
use crate::swc_ecma_ast::AssignOp;
use crate::swc_ecma_ast::BinaryOp;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::ForStmt;
use crate::swc_ecma_ast::Pat;
use crate::swc_ecma_ast::PatOrExpr;
use crate::swc_ecma_ast::UnaryOp;
use crate::swc_ecma_ast::UpdateExpr;
use crate::swc_ecma_ast::UpdateOp;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct ForDirection;

impl LintRule for ForDirection {
  fn new() -> Box<Self> {
    Box::new(ForDirection)
  }

  fn code(&self) -> &'static str {
    "for-direction"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = ForDirectionVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct ForDirectionVisitor {
  context: Context,
}

impl ForDirectionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_update_direction(
    &self,
    update_expr: &UpdateExpr,
    counter_name: &str,
  ) -> i32 {
    let mut update_direction = 0;

    if let Expr::Ident(ident) = &*update_expr.arg {
      if ident.sym.to_string().as_str() == counter_name {
        match update_expr.op {
          UpdateOp::PlusPlus => {
            update_direction = 1;
          }
          UpdateOp::MinusMinus => {
            update_direction = -1;
          }
        }
      }
    }

    update_direction
  }

  fn check_assign_direction(
    &self,
    assign_expr: &AssignExpr,
    counter_name: String,
  ) -> i32 {
    let update_direction = 0;

    let name = match &assign_expr.left {
      PatOrExpr::Expr(boxed_expr) => match &**boxed_expr {
        Expr::Ident(ident) => ident.sym.to_string(),
        _ => return update_direction,
      },
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => return update_direction,
      },
    };

    if name == counter_name {
      return match assign_expr.op {
        AssignOp::AddAssign => {
          self.check_assign_right_direction(assign_expr, 1)
        }
        AssignOp::SubAssign => {
          self.check_assign_right_direction(assign_expr, -1)
        }
        _ => update_direction,
      };
    }
    update_direction
  }

  fn check_assign_right_direction(
    &self,
    assign_expr: &AssignExpr,
    direction: i32,
  ) -> i32 {
    match &*assign_expr.right {
      Expr::Unary(unary_expr) => {
        if unary_expr.op == UnaryOp::Minus {
          -direction
        } else {
          direction
        }
      }
      Expr::Ident(_) => 0,
      _ => direction,
    }
  }
}

impl Visit for ForDirectionVisitor {
  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _parent: &dyn Node) {
    if for_stmt.update.is_none() {
      return;
    }

    if let Some(test) = &for_stmt.test {
      if let Expr::Bin(bin_expr) = &**test {
        let counter_name = match &*bin_expr.left {
          Expr::Ident(ident) => ident.sym.to_string(),
          _ => return,
        };

        let wrong_direction = match &bin_expr.op {
          BinaryOp::Lt | BinaryOp::LtEq => -1,
          BinaryOp::Gt | BinaryOp::GtEq => 1,
          _ => return,
        };

        let update = for_stmt.update.as_ref().unwrap();
        let update_direction = match &**update {
          Expr::Update(update_expr) => {
            self.check_update_direction(update_expr, &counter_name)
          }
          Expr::Assign(assign_expr) => {
            self.check_assign_direction(assign_expr, counter_name)
          }
          _ => return,
        };

        if update_direction == wrong_direction {
          self.context.add_diagnostic(
            for_stmt.span,
            "for-direction",
            "Update clause moves variable in the wrong direction",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn for_direction_ok() {
    assert_lint_ok::<ForDirection>(
      r#"
for(let i = 0; i < 2; i++) {}
for(let i = 0; i <= 2; i++) {}
for(let i = 2; i > 2; i--) {}
for(let i = 2; i >= 0; i--) {}

for(let i = 0; i < 2; i += 1) {}
for(let i = 0; i <= 2; i += 1) {}
for(let i = 0; i < 2; i -= -1) {}
for(let i = 0; i <= 2; i -= -1) {}

for(let i = 2; i > 2; i -= 1) {}
for(let i = 2; i >= 0; i -= 1) {}
for(let i = 2; i > 2; i += -1) {}
for(let i = 2; i >= 0; i += -1) {}

for(let i = 0; i < 2;) {}
for(let i = 0; i <= 2;) {}
for(let i = 2; i > 2;) {}
for(let i = 2; i >= 0;) {}

for(let i = 0; i < 2; i |= 2) {}
for(let i = 0; i <= 2; i %= 2) {}

for(let i = 0; i < 2; j++) {}
for(let i = 0; i <= 2; j--) {}
for(let i = 2; i > 2; j++) {}
for(let i = 2; i >= 0; j--) {}

for(let i = 0; i !== 10; i++) {}
for(let i = 0; i != 10; i++) {}
for(let i = 0; i === 0; i++) {}
for(let i = 0; i == 0; i++) {}
      "#,
    );
  }

  #[test]
  fn for_direction() {
    assert_lint_err::<ForDirection>("for(let i = 0; i < 2; i--) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 0; i <= 2; i--) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i > 2; i++) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i >= 0; i++) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 0; i < 2; i -= 1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 0; i <= 2; i -= 1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i > 2; i -= -1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i >= 0; i -= -1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i > 2; i += 1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 2; i >= 0; i += 1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 0; i < 2; i += -1) {}", 0);
    assert_lint_err::<ForDirection>("for(let i = 0; i <= 2; i += -1) {}", 0);
  }
}
