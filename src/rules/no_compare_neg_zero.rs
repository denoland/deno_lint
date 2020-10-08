// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecmascript::ast::BinaryOp::*;
use swc_ecmascript::ast::Expr::{Lit, Unary};
use swc_ecmascript::ast::Lit::Num;
use swc_ecmascript::ast::UnaryExpr;
use swc_ecmascript::ast::UnaryOp::Minus;
use swc_ecmascript::ast::{BinExpr, BinaryOp, Expr, Module};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct NoCompareNegZero;

impl LintRule for NoCompareNegZero {
  fn new() -> Box<Self> {
    Box::new(NoCompareNegZero)
  }

  fn code(&self) -> &'static str {
    "no-compare-neg-zero"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = NoCompareNegZeroVisitor::new(context);
    visitor.visit_module(module, module);
  }
  fn docs(&self) -> &'static str {
    r#"Disallows comparing against negative zero (`-0`).

Comparing a value directly against negative may not work as expected as it will also pass for non-negative zero (i.e. `0` and `+0`). Explicit comparison with negative zero can be performed using `Object.is`.

### Invalid:
```typescript
if (x === -0) {
}
```
### Valid:
```typescript
if (x === 0) {
}
```
```typescript
if (Object.is(x, -0)) {
}
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

impl<'c> Visit for NoCompareNegZeroVisitor<'c> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    bin_expr.visit_children_with(self);

    if !bin_expr.op.is_comparator() {
      return;
    }

    if bin_expr.left.is_neg_zero() || bin_expr.right.is_neg_zero() {
      self.context.add_diagnostic(
        bin_expr.span,
        "no-compare-neg-zero",
        "Do not compare against -0",
      );
    }
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
  use crate::test_util::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-compare-neg-zero.js
  // MIT Licensed.

  #[test]
  fn no_compare_neg_zero_valid() {
    assert_lint_ok_n::<NoCompareNegZero>(vec![
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
    ]);
  }

  #[test]
  fn no_compare_neg_zero_invalid() {
    assert_lint_err::<NoCompareNegZero>("if (x == -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 == x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x != -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 != x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x === -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 === x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x !== -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 !== x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x < -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 < x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x <= -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 <= x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x > -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 > x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x >= -0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0 >= x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x == -0.0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0.0 == x) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (x === -0.0) { }", 4);
    assert_lint_err::<NoCompareNegZero>("if (-0.0 === x) { }", 4);
    // nested
    assert_lint_err::<NoCompareNegZero>("{} == { foo: x === -0 }", 13);
  }
}
