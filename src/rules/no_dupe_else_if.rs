// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Expression, IfStatement, Program, Statement,
};
use deno_ast::oxc::span::ContentEq;
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;
use deno_ast::oxc::syntax::operator::LogicalOperator;
use derive_more::Display;
use std::collections::HashSet;

#[derive(Debug)]
pub struct NoDupeElseIf;

const CODE: &str = "no-dupe-else-if";

#[derive(Display)]
enum NoDupeElseIfMessage {
  #[display(
    fmt = "This branch can never execute. Its condition is a duplicate or covered by previous conditions in the if-else-if chain."
  )]
  Unexpected,
}

#[derive(Display)]
enum NoDupeElseIfHint {
  #[display(
    fmt = "Remove or rework the `else if` condition which is duplicated"
  )]
  RemoveOrRework,
}

impl LintRule for NoDupeElseIf {
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
    let mut handler = NoDupeElseIfHandler {
      checked_spans: HashSet::new(),
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoDupeElseIfHandler {
  checked_spans: HashSet<Span>,
}

impl Handler<'_> for NoDupeElseIfHandler {
  fn if_statement(&mut self, if_stmt: &IfStatement, ctx: &mut Context) {
    let test_span = if_stmt.test.span();

    // This check is necessary to avoid outputting the same errors multiple times.
    if !self.checked_spans.contains(&test_span) {
      self.checked_spans.insert(test_span);
      let mut appeared_conditions: Vec<Vec<Vec<&Expression>>> = Vec::new();
      append_test(&mut appeared_conditions, &if_stmt.test);

      let mut next = if_stmt.alternate.as_ref();
      while let Some(cur) = next {
        if let Statement::IfStatement(inner_if) = cur {
          let span = inner_if.test.span();
          let mut current_condition_to_check: Vec<Vec<Vec<&Expression>>> =
            mk_condition_to_check(&inner_if.test)
              .into_iter()
              .map(|e| split_by_or_then_and(e))
              .collect();

          for ap_cond in &appeared_conditions {
            current_condition_to_check = current_condition_to_check
              .into_iter()
              .map(|current_or_operands| {
                current_or_operands
                  .into_iter()
                  .filter(|current_or_operand| {
                    !ap_cond.iter().any(|ap_or_operand| {
                      is_subset(ap_or_operand, current_or_operand)
                    })
                  })
                  .collect()
              })
              .collect();

            if current_condition_to_check
              .iter()
              .any(|or_operands| or_operands.is_empty())
            {
              ctx.add_diagnostic_with_hint(
                span,
                CODE,
                NoDupeElseIfMessage::Unexpected,
                NoDupeElseIfHint::RemoveOrRework,
              );
              break;
            }
          }

          self.checked_spans.insert(span);
          append_test(&mut appeared_conditions, &inner_if.test);
          next = inner_if.alternate.as_ref();
        } else {
          break;
        }
      }
    }
  }
}

/// Unwrap parenthesized expressions
fn unwrap_parens<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
  match expr {
    Expression::ParenthesizedExpression(paren) => {
      unwrap_parens(&paren.expression)
    }
    _ => expr,
  }
}

fn mk_condition_to_check<'a>(
  cond: &'a Expression<'a>,
) -> Vec<&'a Expression<'a>> {
  let cond = unwrap_parens(cond);
  match cond {
    Expression::LogicalExpression(logical)
      if logical.operator == LogicalOperator::And =>
    {
      let mut c = vec![cond];
      c.append(&mut split_by_and(cond));
      c
    }
    _ => vec![cond],
  }
}

fn split_by_logical_op<'a>(
  op_to_split: LogicalOperator,
  expr: &'a Expression<'a>,
) -> Vec<&'a Expression<'a>> {
  let expr = unwrap_parens(expr);
  match expr {
    Expression::LogicalExpression(logical) if logical.operator == op_to_split => {
      let mut ret = split_by_logical_op(op_to_split, &logical.left);
      ret.append(&mut split_by_logical_op(op_to_split, &logical.right));
      ret
    }
    _ => vec![expr],
  }
}

fn split_by_or<'a>(expr: &'a Expression<'a>) -> Vec<&'a Expression<'a>> {
  split_by_logical_op(LogicalOperator::Or, expr)
}

fn split_by_and<'a>(expr: &'a Expression<'a>) -> Vec<&'a Expression<'a>> {
  split_by_logical_op(LogicalOperator::And, expr)
}

fn split_by_or_then_and<'a>(
  expr: &'a Expression<'a>,
) -> Vec<Vec<&'a Expression<'a>>> {
  split_by_or(expr).into_iter().map(split_by_and).collect()
}

fn is_subset(arr_a: &[&Expression], arr_b: &[&Expression]) -> bool {
  arr_a
    .iter()
    .all(|a| arr_b.iter().any(|b| equal_in_if_else(a, b)))
}

/// Determines whether the two given expressions are considered equal
/// in if-else condition context. Uses ContentEq for structural comparison
/// (ignoring spans), with special handling for logical operators where
/// operand order doesn't matter.
fn equal_in_if_else(expr1: &Expression, expr2: &Expression) -> bool {
  let expr1 = unwrap_parens(expr1);
  let expr2 = unwrap_parens(expr2);

  // Special case for logical AND/OR: operand order doesn't matter
  if let (
    Expression::LogicalExpression(log1),
    Expression::LogicalExpression(log2),
  ) = (expr1, expr2)
  {
    if matches!(
      log1.operator,
      LogicalOperator::Or | LogicalOperator::And
    ) && log1.operator == log2.operator
    {
      return (equal_in_if_else(&log1.left, &log2.left)
        && equal_in_if_else(&log1.right, &log2.right))
        || (equal_in_if_else(&log1.left, &log2.right)
          && equal_in_if_else(&log1.right, &log2.left));
    }
  }

  expr1.content_eq(expr2)
}

fn append_test<'a>(
  appeared_conditions: &mut Vec<Vec<Vec<&'a Expression<'a>>>>,
  expr: &'a Expression<'a>,
) {
  appeared_conditions.push(split_by_or_then_and(expr));
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_dupe_else_if_valid() {
    assert_lint_ok! {
      NoDupeElseIf,
      "if (a) {} else if (b) {}",
      "if (a); else if (b); else if (c);",
      "if (true) {} else if (false) {} else {}",
      "if (1) {} else if (2) {}",
      "if (f) {} else if (f()) {}",
      "if (f(a)) {} else if (g(a)) {}",
      "if (f(a)) {} else if (f(b)) {}",
      "if (a === 1) {} else if (a === 2) {}",
      "if (a === 1) {} else if (b === 1) {}",
      "if (a) {}",
      "if (a);",
      "if (a) {} else {}",
      "if (a) if (a) {}",
      "if (a) if (a);",
      "if (a) { if (a) {} }",
      r#"
if (a) {}
else {
  if (a) {}
}
"#,
      "if (a) {} if (a) {}",
      "if (a); if (a);",
      "while (a) if (a);",
      "if (a); else a ? a : a;",
      r#"
if (a) {
  if (b) {}
} else if (b) {}
"#,
      r#"
if (a) {
  if (b !== 1) {}
  else if (b !== 2) {}
} else if (c) {}
"#,
      "if (a) if (b); else if (a);",
      "if (a) {} else if (!!a) {}",
      "if (a === 1) {} else if (a === (1)) {}",
      "if (a || b) {} else if (c || d) {}",
      "if (a || b) {} else if (a || c) {}",
      "if (a) {} else if (a || b) {}",
      "if (a) {} else if (b) {} else if (a || b || c) {}",
      "if (a && b) {} else if (a) {} else if (b) {}",
      "if (a && b) {} else if (b && c) {} else if (a && c) {}",
      "if (a && b) {} else if (b || c) {}",
      "if (a) {} else if (b && (a || c)) {}",
      "if (a) {} else if (b && (c || d && a)) {}",
      "if (a && b && c) {} else if (a && b && (c || d)) {}",
    };
  }

  #[test]
  fn no_dupe_else_if_invalid() {
    assert_lint_err! {
      NoDupeElseIf,
      "if (a) {} else if (a) {} else if (b) {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a); else if (a);": [
        {
          col: 17,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (a) {} else {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (a) {} else if (c) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (a) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (c) {} else if (a) {}": [
        {
          col: 49,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (b) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (b) {} else {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (c) {} else if (b) {}": [
        {
          col: 49,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a); else if (b); else if (c); else if (b); else if (d); else;": [
        {
          col: 43,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a); else if (b); else if (c); else if (d); else if (b); else if (e);": [
        {
          col: 56,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (a) {} else if (a) {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        },
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (a) {} else if (b) {} else if (a) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        },
        {
          col: 49,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        },
        {
          col: 64,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) { if (b) {} } else if (a) {}": [
        {
          col: 30,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (this) {} else if (this) {}": [
        {
          col: 22,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if ([a]) {} else if ([a]) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if ({a: 1}) {} else if ({a: 1}) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (function () {}) {} else if (function () {}) {}": [
        {
          col: 32,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (!a) {} else if (!a) {}": [
        {
          col: 20,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (++a) {} else if (++a) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a === 1) {} else if (a === 1) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (1 < a) {} else if (1 < a) {}": [
        {
          col: 23,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a = b) {} else if (a = b) {}": [
        {
          col: 23,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a.b) {} else if (a.b) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a[b]) {} else if (a[b]) {}": [
        {
          col: 22,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a ? 1 : 2) {} else if (a ? 1 : 2) {}": [
        {
          col: 27,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a()) {} else if (a()) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (new A()) {} else if (new A()) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (true) {} else if (true) {}": [
        {
          col: 22,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (`a`) {} else if (`a`) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a`b`) {} else if (a`b`) {}": [
        {
          col: 22,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a => {}) {} else if (a => {}) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (class A {}) {} else if (class A {}) {}": [
        {
          col: 28,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      r#"
function* foo(a) {
  if (yield a) {}
  else if (yield a) {}
}
      "#: [
        {
          line: 4,
          col: 11,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (new.target) {} else if (new.target) {}": [
        {
          col: 28,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (await a) {} else if (await a) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if ((a)) {} else if ((a)) {}": [
        {
          col: 21,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && b) {} else if (a && b) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && b || c)  {} else if (a && b || c) {}": [
        {
          col: 30,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (f(a)) {} else if (f(a)) {}": [
        {
          col: 22,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a === 1) {} else if (a===1) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a === 1) {} else if (a === /* comment */ 1) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a === 1) {} else if ((a === 1)) {}": [
        {
          col: 25,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (a) {} else if (b) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        },
        {
          col: 39,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (b || a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (a || b) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (c || d) {} else if (a || d) {}": [
        {
          col: 44,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if ((a === b && fn(c)) || d) {} else if (fn(c) && a === b) {}": [
        {
          col: 41,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (a && b) {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && b) {} else if (b && a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && b) {} else if (a && b && c) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || c) {} else if (a && b || c) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (c && a || b) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (c && (a || b)) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b && c) {} else if (d && (a || e && c && b)) {}": [
        {
          col: 39,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b && c) {} else if (b && c && d) {}": [
        {
          col: 29,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (b && c) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if ((a || b) && c) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if ((a && (b || c)) || d) {} else if ((c || b) && e && a) {}": [
        {
          col: 38,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && b || b && c) {} else if (a && b && c) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b && c) {} else if (d && (c && e && b || a)) {}": [
        {
          col: 39,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || (b && (c || d))) {} else if ((d || c) && b) {}": [
        {
          col: 38,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if ((b || a) && c) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (c) {} else if (d) {} else if (b && (a || c)) {}": [
        {
          col: 54,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b || c) {} else if (a || (b && d) || (c && e)) {}": [
        {
          col: 29,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || (b || c)) {} else if (a || (b && c)) {}": [
        {
          col: 31,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || b) {} else if (c) {} else if (d) {} else if ((a || c) && (b || d)) {}": [
        {
          col: 54,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (b) {} else if (c && (a || d && b)) {}": [
        {
          col: 34,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (a || a) {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || a) {} else if (a || a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a || a) {} else if (a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a) {} else if (a && a) {}": [
        {
          col: 19,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && a) {} else if (a && a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      "if (a && a) {} else if (a) {}": [
        {
          col: 24,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],

      // nested
      r#"
if (foo) {
  if (a == 1) {}
  else if (a == 1) {}
}
      "#: [
        {
          line: 4,
          col: 11,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ],
      r#"
if (foo) {
  if (a == 1) {}
  else if (a > 1) {}
} else if (foo) {}
      "#: [
        {
          line: 5,
          col: 11,
          message: NoDupeElseIfMessage::Unexpected,
          hint: NoDupeElseIfHint::RemoveOrRework,
        }
      ]
    };
  }
}
