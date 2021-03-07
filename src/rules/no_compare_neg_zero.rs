// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_ecmascript::ast::BinaryOp::*;
use swc_ecmascript::ast::Expr::{Lit, Unary};
use swc_ecmascript::ast::Lit::Num;
use swc_ecmascript::ast::UnaryExpr;
use swc_ecmascript::ast::UnaryOp::Minus;
use swc_ecmascript::ast::{BinExpr, BinaryOp, Expr};
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

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

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoCompareNegZeroVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows comparing against negative zero (`-0`).

Comparing a value directly against negative may not work as expected as it will also pass for non-negative zero (i.e. `0` and `+0`). Explicit comparison with negative zero can be performed using `Object.is`.

### Invalid:
```typescript
if (x === -0) {}
```

### Valid:
```typescript
if (x === 0) {}

if (Object.is(x, -0)) {}
```"#
  }
}

struct NoCompareNegZeroVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoCompareNegZeroVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> VisitAll for NoCompareNegZeroVisitor<'c> {
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
