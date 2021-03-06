// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::AssignOp;
use swc_ecmascript::ast::BinaryOp;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ForStmt;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::PatOrExpr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::ast::UpdateExpr;
use swc_ecmascript::ast::UpdateOp;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct ForDirection;

impl LintRule for ForDirection {
  fn new() -> Box<Self> {
    Box::new(ForDirection)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "for-direction"
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = ForDirectionVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Requires `for` loop control variables to increment in the correct direction

Incrementing `for` loop control variables in the wrong direction leads to infinite
loops.  This can occur through incorrect initialization, bad continuation step logic
or wrong direction incrementing of the loop control variable.  

### Invalid:
```typescript
// Infinite loop
for(let i = 0; i < 2; i--) {}
```

### Valid:
```typescript
for(let i = 0; i < 2; i++) {}
```
"#
  }
}

const MESSAGE: &str = "Update clause moves variable in the wrong direction";
const HINT: &str =
  "Flip the update clause logic or change the continuation step condition";

struct ForDirectionVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ForDirectionVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_update_direction(
    &self,
    update_expr: &UpdateExpr,
    counter_name: impl AsRef<str>,
  ) -> i32 {
    let mut update_direction = 0;

    if let Expr::Ident(ident) = &*update_expr.arg {
      if ident.sym.as_ref() == counter_name.as_ref() {
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
    counter_name: impl AsRef<str>,
  ) -> i32 {
    let update_direction = 0;

    let name = match &assign_expr.left {
      PatOrExpr::Expr(boxed_expr) => match &**boxed_expr {
        Expr::Ident(ident) => ident.sym.as_ref(),
        _ => return update_direction,
      },
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => ident.id.sym.as_ref(),
        _ => return update_direction,
      },
    };

    if name == counter_name.as_ref() {
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

impl<'c> VisitAll for ForDirectionVisitor<'c> {
  noop_visit_type!();

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _parent: &dyn Node) {
    if for_stmt.update.is_none() {
      return;
    }

    if let Some(test) = &for_stmt.test {
      if let Expr::Bin(bin_expr) = &**test {
        let counter_name = match &*bin_expr.left {
          Expr::Ident(ident) => ident.sym.as_ref(),
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
            self.check_update_direction(update_expr, counter_name)
          }
          Expr::Assign(assign_expr) => {
            self.check_assign_direction(assign_expr, counter_name)
          }
          _ => return,
        };

        if update_direction == wrong_direction {
          self.context.add_diagnostic_with_hint(
            for_stmt.span,
            "for-direction",
            MESSAGE,
            HINT,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn for_direction_valid() {
    assert_lint_ok! {
      ForDirection,
      "for(let i = 0; i < 2; i++) {}",
      "for(let i = 0; i < 2; ++i) {}",
      "for(let i = 0; i <= 2; i++) {}",
      "for(let i = 0; i <= 2; ++i) {}",
      "for(let i = 2; i > 2; i--) {}",
      "for(let i = 2; i > 2; --i) {}",
      "for(let i = 2; i >= 0; i--) {}",
      "for(let i = 2; i >= 0; --i) {}",
      "for(let i = 0; i < 2; i += 1) {}",
      "for(let i = 0; i <= 2; i += 1) {}",
      "for(let i = 0; i < 2; i -= -1) {}",
      "for(let i = 0; i <= 2; i -= -1) {}",
      "for(let i = 2; i > 2; i -= 1) {}",
      "for(let i = 2; i >= 0; i -= 1) {}",
      "for(let i = 2; i > 2; i += -1) {}",
      "for(let i = 2; i >= 0; i += -1) {}",
      "for(let i = 0; i < 2;) {}",
      "for(let i = 0; i <= 2;) {}",
      "for(let i = 2; i > 2;) {}",
      "for(let i = 2; i >= 0;) {}",
      "for(let i = 0; i < 2; i |= 2) {}",
      "for(let i = 0; i <= 2; i %= 2) {}",
      "for(let i = 0; i < 2; j++) {}",
      "for(let i = 0; i <= 2; j--) {}",
      "for(let i = 2; i > 2; j++) {}",
      "for(let i = 2; i >= 0; j--) {}",
      "for(let i = 0; i !== 10; i++) {}",
      "for(let i = 0; i != 10; i++) {}",
      "for(let i = 0; i === 0; i++) {}",
      "for(let i = 0; i == 0; i++) {}",
      "for(let i = 0; i < 2; ++i) { for (let j = 0; j < 2; j++) {} }",
    };
  }

  #[test]
  fn for_direction_invalid() {
    assert_lint_err! {
      ForDirection,

      // ++, --
      "for(let i = 0; i < 2; i--) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i < 2; --i) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i <= 2; i--) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i <= 2; --i) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i > 2; i++) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i > 2; ++i) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i >= 0; i++) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i >= 0; ++i) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],

      // +=, -=
      "for(let i = 0; i < 2; i -= 1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i <= 2; i -= 1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i > 2; i -= -1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i >= 0; i -= -1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i > 2; i += 1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 2; i >= 0; i += 1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i < 2; i += -1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "for(let i = 0; i <= 2; i += -1) {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],

      // nested
      r#"
for (let i = 0; i < 2; i++) {
  for (let j = 0; j < 2; j--) {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
