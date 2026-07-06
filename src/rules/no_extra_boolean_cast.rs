// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  CallExpression, ConditionalExpression, DoWhileStatement, Expression,
  ForStatement, IfStatement, LogicalExpression, NewExpression, Program,
  UnaryExpression, UnaryOperator, WhileStatement,
};
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct NoExtraBooleanCast;

const CODE: &str = "no-extra-boolean-cast";

#[derive(Display)]
enum NoExtraBooleanCastMessage {
  #[display(fmt = "Redundant Boolean call.")]
  BooleanCall,
  #[display(fmt = "Redundant double negation.")]
  DoubleNegation,
}

#[derive(Display)]
enum NoExtraBooleanCastHint {
  #[display(fmt = "Remove the Boolean call, it is unnecessary")]
  BooleanCall,
  #[display(fmt = "Remove the double negation (`!!`), it is unnecessary")]
  DoubleNegation,
}

impl LintRule for NoExtraBooleanCast {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoExtraBooleanCastHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoExtraBooleanCastHandler;

fn unexpected_call(span: Span, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoExtraBooleanCastMessage::BooleanCall,
    NoExtraBooleanCastHint::BooleanCall,
  );
}

fn unexpected_negation(span: Span, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoExtraBooleanCastMessage::DoubleNegation,
    NoExtraBooleanCastHint::DoubleNegation,
  );
}

fn check_condition(expr: &Expression, ctx: &mut Context) {
  match expr {
    Expression::CallExpression(call_expr) => {
      if callee_is_boolean(&call_expr.callee) {
        unexpected_call(call_expr.span, ctx);
      }
    }
    Expression::UnaryExpression(unary_expr)
      if unary_expr.operator == UnaryOperator::LogicalNot
        && has_n_bang(&unary_expr.argument, 1) =>
    {
      unexpected_negation(unary_expr.span, ctx);
    }
    Expression::ParenthesizedExpression(paren_expr) => {
      check_condition(&paren_expr.expression, ctx);
    }
    Expression::LogicalExpression(logical_expr)
      // If ALL parts of a logical expression are redundant boolean casts,
      // report each one. E.g. `if (!!foo || !!bar) {}` → report both.
      // But `if (!!foo || bar) {}` → report nothing (bar is not a cast).
      if all_parts_are_redundant_cast(logical_expr) => {
        report_all_parts(logical_expr, ctx);
      }
    _ => (),
  }
}

/// Returns true if all leaf expressions in a logical expression tree
/// (connected by `||` or `&&`) are redundant boolean casts (`!!` or `Boolean()`).
fn all_parts_are_redundant_cast(logical_expr: &LogicalExpression) -> bool {
  is_redundant_cast(&logical_expr.left)
    && is_redundant_cast(&logical_expr.right)
}

fn is_redundant_cast(expr: &Expression) -> bool {
  match expr {
    Expression::CallExpression(c) => callee_is_boolean(&c.callee),
    Expression::UnaryExpression(u)
      if u.operator == UnaryOperator::LogicalNot
        && has_n_bang(&u.argument, 1) =>
    {
      true
    }
    Expression::LogicalExpression(inner) => all_parts_are_redundant_cast(inner),
    Expression::ParenthesizedExpression(p) => is_redundant_cast(&p.expression),
    _ => false,
  }
}

fn report_all_parts(logical_expr: &LogicalExpression, ctx: &mut Context) {
  report_redundant_cast(&logical_expr.left, ctx);
  report_redundant_cast(&logical_expr.right, ctx);
}

fn report_redundant_cast(expr: &Expression, ctx: &mut Context) {
  match expr {
    Expression::CallExpression(c) if callee_is_boolean(&c.callee) => {
      unexpected_call(c.span, ctx);
    }
    Expression::UnaryExpression(u)
      if u.operator == UnaryOperator::LogicalNot
        && has_n_bang(&u.argument, 1) =>
    {
      unexpected_negation(u.span, ctx);
    }
    Expression::LogicalExpression(inner) => {
      report_all_parts(inner, ctx);
    }
    Expression::ParenthesizedExpression(p) => {
      report_redundant_cast(&p.expression, ctx);
    }
    _ => {}
  }
}

fn check_unary_expr(unary_expr: &UnaryExpression, ctx: &mut Context) {
  if unary_expr.operator == UnaryOperator::LogicalNot {
    let expr = &unary_expr.argument;
    check_unary_expr_internal(unary_expr.span, expr, ctx);
  }
}

fn check_unary_expr_internal(
  unary_expr_span: Span,
  internal_expr: &Expression,
  ctx: &mut Context,
) {
  match internal_expr {
    Expression::CallExpression(call_expr) => {
      if callee_is_boolean(&call_expr.callee) {
        unexpected_call(unary_expr_span, ctx);
      }
    }
    Expression::UnaryExpression(unary_expr)
      if unary_expr.operator == UnaryOperator::LogicalNot
        && has_n_bang(&unary_expr.argument, 1) =>
    {
      unexpected_negation(unary_expr_span, ctx);
    }
    Expression::ParenthesizedExpression(paren_expr) => {
      check_unary_expr_internal(unary_expr_span, &paren_expr.expression, ctx);
    }
    _ => (),
  }
}

impl Handler<'_> for NoExtraBooleanCastHandler {
  fn conditional_expression(
    &mut self,
    cond_expr: &ConditionalExpression,
    ctx: &mut Context,
  ) {
    check_condition(&cond_expr.test, ctx);
  }

  fn for_statement(&mut self, for_stmt: &ForStatement, ctx: &mut Context) {
    if let Some(ref test_expr) = for_stmt.test {
      check_condition(test_expr, ctx);
    }
  }

  fn if_statement(&mut self, if_stmt: &IfStatement, ctx: &mut Context) {
    check_condition(&if_stmt.test, ctx);
  }

  fn while_statement(
    &mut self,
    while_stmt: &WhileStatement,
    ctx: &mut Context,
  ) {
    check_condition(&while_stmt.test, ctx);
  }

  fn do_while_statement(
    &mut self,
    do_while_stmt: &DoWhileStatement,
    ctx: &mut Context,
  ) {
    check_condition(&do_while_stmt.test, ctx);
  }

  fn call_expression(&mut self, call_expr: &CallExpression, ctx: &mut Context) {
    if callee_is_boolean(&call_expr.callee) {
      if let Some(arg) = call_expr.arguments.first() {
        if let Some(expr) = arg.as_expression() {
          check_condition(expr, ctx);
        }
      }
    }
  }

  fn new_expression(&mut self, new_expr: &NewExpression, ctx: &mut Context) {
    if expr_callee_is_boolean(&new_expr.callee) {
      if let Some(arg) = new_expr.arguments.first() {
        if let Some(expr) = arg.as_expression() {
          check_condition(expr, ctx);
        }
      }
    }
  }

  fn unary_expression(
    &mut self,
    unary_expr: &UnaryExpression,
    ctx: &mut Context,
  ) {
    check_unary_expr(unary_expr, ctx);
  }
}

fn callee_is_boolean(callee: &Expression) -> bool {
  expr_callee_is_boolean(callee)
}

fn expr_callee_is_boolean(expr: &Expression) -> bool {
  matches!(expr, Expression::Identifier(ident) if ident.name.as_str() == "Boolean")
}

/// Checks if `expr` has `n` continuous bang operators at the beginning, ignoring parentheses.
fn has_n_bang(expr: &Expression, n: usize) -> bool {
  if n == 0 {
    return true;
  }

  match expr {
    Expression::UnaryExpression(unary_expr) => {
      if unary_expr.operator == UnaryOperator::LogicalNot {
        has_n_bang(&unary_expr.argument, n - 1)
      } else {
        false
      }
    }
    Expression::ParenthesizedExpression(paren_expr) => {
      has_n_bang(&paren_expr.expression, n)
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_extra_boolean_cast_valid() {
    assert_lint_ok! {
      NoExtraBooleanCast,
      "Boolean(bar, !!baz);",
      "var foo = !!bar;",
      "function foo() { return !!bar; }",
      "var foo = bar() ? !!baz : !!bat",
      "for(!!foo;;) {}",
      "for(;; !!foo) {}",
      "var foo = Boolean(bar);",
      "function foo() { return Boolean(bar); }",
      "var foo = bar() ? Boolean(baz) : Boolean(bat)",
      "for(Boolean(foo);;) {}",
      "for(;; Boolean(foo)) {}",
      "if (new Boolean(foo)) {}",
      "if (!!foo || bar) {}",
    };
  }

  #[test]
  fn no_extra_boolean_cast_invalid() {
    assert_lint_err! {
      NoExtraBooleanCast,
      "if (!!foo) {}": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (!!foo)": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (!!foo) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!foo ? bar : baz": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; !!foo;) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!foo": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!!foo)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!foo)": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(foo)) {}": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (Boolean(foo))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (Boolean(foo)) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(foo) ? bar : baz": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; Boolean(foo);) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo && bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo + bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo || bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(Boolean(foo))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 9,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(Boolean(foo))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!!foo || !!bar) {}": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        },
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
    };
  }
}
