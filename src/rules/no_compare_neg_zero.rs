// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::BinaryOp::*;
use deno_ast::swc::ast::Expr::{Lit, Unary};
use deno_ast::swc::ast::Lit::Num;
use deno_ast::swc::ast::UnaryExpr;
use deno_ast::swc::ast::UnaryOp::Minus;
use deno_ast::swc::ast::{BinExpr, BinaryOp, Expr};
use deno_ast::swc::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};
use derive_more::Display;

pub struct NoCompareNegZero;

const CODE: &str = "no-compare-neg-zero";

#[derive(Display)]
enum NoCompareNegZeroMessage {
  #[display(fmt = NoCompareNegZeroMessage::Unexpected)]
  Unexpected,
}

#[derive(Display)]
enum NoCompareNegZeroHint {
  #[display(
    fmt = NoCompareNegZeroHint::ObjectIs
  )]
  ObjectIs,
}

impl LintRule for NoCompareNegZero {
  fn new() -> Box<Self> {
    Box::new(NoCompareNegZero)
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
    let mut visitor = NoCompareNegZeroVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_compare_neg_zero.md")
  }
}

struct NoCompareNegZeroVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoCompareNegZeroVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoCompareNegZeroVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if !bin_expr.op.is_comparator() {
      return;
    }

    if bin_expr.left.is_neg_zero() || bin_expr.right.is_neg_zero() {
      self.context.add_diagnostic_with_hint(
        bin_expr.span,
        CODE,
        NoCompareNegZeroMessage::Unexpected,
        NoCompareNegZeroHint::ObjectIs,
      );
    }
  }
}

trait Comparator {
  fn is_comparator(&self) -> bool;
}

impl Comparator for BinaryOp {
  fn is_comparator(&self) -> bool {
    matches!(
      self,
      EqEq | NotEq | EqEqEq | NotEqEq | Lt | LtEq | Gt | GtEq
    )
  }
}

trait NegZero {
  fn is_neg_zero(&self) -> bool;
}

impl NegZero for Expr {
  fn is_neg_zero(&self) -> bool {
    match self {
      Unary(unary) => unary.is_neg_zero(),
      _ => false,
    }
  }
}

impl NegZero for UnaryExpr {
  fn is_neg_zero(&self) -> bool {
    if let (Minus, Lit(Num(number))) = (self.op, &*self.arg) {
      return number.value == 0.0;
    }
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-compare-neg-zero.js
  // MIT Licensed.

  #[test]
  fn no_compare_neg_zero_valid() {
    assert_lint_ok! {
      NoCompareNegZero,
      r#"if (x === 0) { }"#,
      r#"if (Object.is(x, -0)) { }"#,
      r#"x === 0"#,
      r#"0 === x"#,
      r#"x == 0"#,
      r#"0 == x"#,
      r#"x === '0'"#,
      r#"'0' === x"#,
      r#"x == '0'"#,
      r#"'0' == x"#,
      r#"x === '-0'"#,
      r#"'-0' === x"#,
      r#"x == '-0'"#,
      r#"'-0' == x"#,
      r#"x === -1"#,
      r#"-1 === x"#,
      r#"x < 0"#,
      r#"0 < x"#,
      r#"x <= 0"#,
      r#"0 <= x"#,
      r#"x > 0"#,
      r#"0 > x"#,
      r#"x >= 0"#,
      r#"0 >= x"#,
      r#"x != 0"#,
      r#"0 != x"#,
      r#"x !== 0"#,
      r#"0 !== x"#,
      r#"{} == { foo: x === 0 }"#,
    };
  }

  #[test]
  fn no_compare_neg_zero_invalid() {
    assert_lint_err! {
      NoCompareNegZero,
      "if (x == -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x != -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 != x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x === -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x !== -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 !== x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x < -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 < x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x <= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 <= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x > -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 > x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x >= -0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0 >= x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x == -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0.0 == x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (x === -0.0) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],
      "if (-0.0 === x) { }": [
        {
          col: 4,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ],

      // nested
      "{} == { foo: x === -0 }": [
        {
          col: 13,
          message: NoCompareNegZeroMessage::Unexpected,
          hint: NoCompareNegZeroHint::ObjectIs,
        }
      ]
    };
  }
}
