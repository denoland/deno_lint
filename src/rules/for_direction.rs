// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct ForDirection;

impl LintRule for ForDirection {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    "for-direction"
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = ForDirectionHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Update clause moves variable in the wrong direction";
const HINT: &str =
  "Flip the update clause logic or change the continuation step condition";

struct ForDirectionHandler;

fn check_update_direction(
  update_expr: &UpdateExpression,
  counter_name: impl AsRef<str>,
) -> i32 {
  let mut update_direction = 0;

  if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) =
    &update_expr.argument
  {
    if ident.name.as_str() == counter_name.as_ref() {
      match update_expr.operator {
        UpdateOperator::Increment => {
          update_direction = 1;
        }
        UpdateOperator::Decrement => {
          update_direction = -1;
        }
      }
    }
  }

  update_direction
}

fn check_assign_direction(
  assign_expr: &AssignmentExpression,
  counter_name: impl AsRef<str>,
) -> i32 {
  let update_direction = 0;

  let name = match &assign_expr.left {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => ident.name.as_str(),
    _ => return update_direction,
  };

  if name == counter_name.as_ref() {
    return match assign_expr.operator {
      AssignmentOperator::Addition => {
        check_assign_right_direction(assign_expr, 1)
      }
      AssignmentOperator::Subtraction => {
        check_assign_right_direction(assign_expr, -1)
      }
      _ => update_direction,
    };
  }
  update_direction
}

fn check_assign_right_direction(
  assign_expr: &AssignmentExpression,
  direction: i32,
) -> i32 {
  match &assign_expr.right {
    Expression::UnaryExpression(unary_expr) => {
      if unary_expr.operator == UnaryOperator::UnaryNegation {
        -direction
      } else {
        direction
      }
    }
    Expression::Identifier(_) => 0,
    _ => direction,
  }
}

impl Handler<'_> for ForDirectionHandler {
  fn for_statement(&mut self, for_stmt: &ForStatement, context: &mut Context) {
    if for_stmt.update.is_none() {
      return;
    }

    if let Some(Expression::BinaryExpression(bin_expr)) = &for_stmt.test {
      let counter_name = match &bin_expr.left {
        Expression::Identifier(ident) => ident.name.as_str(),
        _ => return,
      };

      let wrong_direction = match bin_expr.operator {
        BinaryOperator::LessThan | BinaryOperator::LessEqualThan => -1,
        BinaryOperator::GreaterThan | BinaryOperator::GreaterEqualThan => 1,
        _ => return,
      };

      let update = for_stmt.update.as_ref().unwrap();
      let update_direction = match update {
        Expression::UpdateExpression(update_expr) => {
          check_update_direction(update_expr, counter_name)
        }
        Expression::AssignmentExpression(assign_expr) => {
          check_assign_direction(assign_expr, counter_name)
        }
        _ => return,
      };

      if update_direction == wrong_direction {
        context.add_diagnostic_with_hint(
          for_stmt.span,
          "for-direction",
          MESSAGE,
          HINT,
        );
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
