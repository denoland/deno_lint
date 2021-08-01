// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use std::collections::HashSet;
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{BinExpr, BinaryOp, Expr, IfStmt, ParenExpr, Stmt};
use swc_ecmascript::utils::drop_span;
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

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
  fn new() -> Box<Self> {
    Box::new(NoDupeElseIf)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoDupeElseIfVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_dupe_else_if.md")
  }
}

/// A visitor to check the `no-dupe-else-if` rule.
/// Determination logic is ported from ESLint's implementation. For more, see:
/// [eslint/no-dupe-else-if.js](https://github.com/eslint/eslint/blob/master/lib/rules/no-dupe-else-if.js).
struct NoDupeElseIfVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  checked_span: HashSet<Span>,
}

impl<'c, 'view> NoDupeElseIfVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self {
      context,
      checked_span: HashSet::new(),
    }
  }
}

impl<'c, 'view> VisitAll for NoDupeElseIfVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _: &dyn Node) {
    let span = if_stmt.test.span();

    // This check is necessary to avoid outputting the same errors multiple times.
    if !self.checked_span.contains(&span) {
      self.checked_span.insert(span);
      let span_dropped_test = drop_span(if_stmt.test.clone());
      let mut appeared_conditions: Vec<Vec<Vec<Expr>>> = Vec::new();
      append_test(&mut appeared_conditions, *span_dropped_test);

      let mut next = if_stmt.alt.as_ref();
      while let Some(cur) = next {
        if let Stmt::If(IfStmt {
          ref test, ref alt, ..
        }) = &**cur
        {
          // preserve the span before dropping
          let span = test.span();
          let span_dropped_test = drop_span(test.clone());
          let mut current_condition_to_check: Vec<Vec<Vec<Expr>>> =
            mk_condition_to_check(*span_dropped_test.clone())
              .into_iter()
              .map(split_by_or_then_and)
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
              self.context.add_diagnostic_with_hint(
                span,
                CODE,
                NoDupeElseIfMessage::Unexpected,
                NoDupeElseIfHint::RemoveOrRework,
              );
              break;
            }
          }

          self.checked_span.insert(span);
          append_test(&mut appeared_conditions, *span_dropped_test);
          next = alt.as_ref();
        } else {
          break;
        }
      }
    }
  }
}

fn mk_condition_to_check(cond: Expr) -> Vec<Expr> {
  match cond {
    Expr::Bin(BinExpr { op, .. }) if op == BinaryOp::LogicalAnd => {
      let mut c = vec![cond.clone()];
      c.append(&mut split_by_and(cond));
      c
    }
    Expr::Paren(ParenExpr { expr, .. }) => mk_condition_to_check(*expr),
    _ => vec![cond],
  }
}

fn split_by_bin_op(op_to_split: BinaryOp, expr: Expr) -> Vec<Expr> {
  match expr {
    Expr::Bin(BinExpr {
      op, left, right, ..
    }) if op == op_to_split => {
      let mut ret = split_by_bin_op(op_to_split, *left);
      ret.append(&mut split_by_bin_op(op_to_split, *right));
      ret
    }
    Expr::Paren(ParenExpr { expr, .. }) => split_by_bin_op(op_to_split, *expr),
    _ => vec![expr],
  }
}

fn split_by_or(expr: Expr) -> Vec<Expr> {
  split_by_bin_op(BinaryOp::LogicalOr, expr)
}

fn split_by_and(expr: Expr) -> Vec<Expr> {
  split_by_bin_op(BinaryOp::LogicalAnd, expr)
}

fn split_by_or_then_and(expr: Expr) -> Vec<Vec<Expr>> {
  split_by_or(expr).into_iter().map(split_by_and).collect()
}

fn is_subset(arr_a: &[Expr], arr_b: &[Expr]) -> bool {
  arr_a
    .iter()
    .all(|a| arr_b.iter().any(|b| equal_in_if_else(a, b)))
}

/// Determines whether the two given `Expr`s are considered to be equal in if-else condition
/// context. Note that `expr1` and `expr2` must be span-dropped to be compared properly.
fn equal_in_if_else(expr1: &Expr, expr2: &Expr) -> bool {
  use swc_ecmascript::ast::Expr::*;
  match (expr1, expr2) {
    (Bin(ref bin1), Bin(ref bin2))
      if matches!(bin1.op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd)
        && bin1.op == bin2.op =>
    {
      equal_in_if_else(&*bin1.left, &*bin2.left)
        && equal_in_if_else(&*bin1.right, &*bin2.right)
        || equal_in_if_else(&*bin1.left, &*bin2.right)
          && equal_in_if_else(&*bin1.right, &*bin2.left)
    }
    (Paren(ParenExpr { ref expr, .. }), _) => equal_in_if_else(&**expr, expr2),
    (_, Paren(ParenExpr { ref expr, .. })) => equal_in_if_else(expr1, &**expr),
    (This(_), This(_))
    | (Array(_), Array(_))
    | (Object(_), Object(_))
    | (Fn(_), Fn(_))
    | (Unary(_), Unary(_))
    | (Update(_), Update(_))
    | (Bin(_), Bin(_))
    | (Assign(_), Assign(_))
    | (Member(_), Member(_))
    | (Cond(_), Cond(_))
    | (Call(_), Call(_))
    | (New(_), New(_))
    | (Seq(_), Seq(_))
    | (Ident(_), Ident(_))
    | (Lit(_), Lit(_))
    | (Tpl(_), Tpl(_))
    | (TaggedTpl(_), TaggedTpl(_))
    | (Arrow(_), Arrow(_))
    | (Class(_), Class(_))
    | (Yield(_), Yield(_))
    | (MetaProp(_), MetaProp(_))
    | (Await(_), Await(_))
    | (JSXMember(_), JSXMember(_))
    | (JSXNamespacedName(_), JSXNamespacedName(_))
    | (JSXEmpty(_), JSXEmpty(_))
    | (JSXElement(_), JSXElement(_))
    | (JSXFragment(_), JSXFragment(_))
    | (TsTypeAssertion(_), TsTypeAssertion(_))
    | (TsConstAssertion(_), TsConstAssertion(_))
    | (TsNonNull(_), TsNonNull(_))
    | (TsAs(_), TsAs(_))
    | (PrivateName(_), PrivateName(_))
    | (OptChain(_), OptChain(_))
    | (Invalid(_), Invalid(_)) => expr1 == expr2,
    _ => false,
  }
}

fn append_test(appeared_conditions: &mut Vec<Vec<Vec<Expr>>>, expr: Expr) {
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
