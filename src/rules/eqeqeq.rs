// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{BinExpr, BinaryOp};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct Eqeqeq;

impl LintRule for Eqeqeq {
  fn new() -> Box<Self> {
    Box::new(Eqeqeq)
  }

  fn code(&self) -> &'static str {
    "eqeqeq"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = EqeqeqVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn docs(&self) -> &'static str {
    r#"Enforces the use of type-safe equality operators `===` and `!==`
instead of the more error prone `==` and `!=` operators.

`===` and `!==` ensure the comparators are of the same type as well as the same
value.  On the other hand `==` and `!=` do type coercion before value checking
which can lead to unexpected results.  For example `5 == "5"` is true, while
`5 === "5"` is false.

### Invalid:
```typescript
if (a == 5) {}
if ("hello world" != input) {}
```

### Valid:
```typescript
if (a === 5) {}
if ("hello world" !== input) {}
```
"#
  }
}

struct EqeqeqVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> EqeqeqVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for EqeqeqVisitor<'c> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, parent: &dyn Node) {
    if matches!(bin_expr.op, BinaryOp::EqEq | BinaryOp::NotEq) {
      let message = if bin_expr.op == BinaryOp::EqEq {
        "expected '===' and instead saw '=='."
      } else {
        "expected '!==' and instead saw '!='."
      };
      let hint = if bin_expr.op == BinaryOp::EqEq {
        "Use '==='"
      } else {
        "Use '!=='"
      };
      self.context.add_diagnostic_with_hint(
        bin_expr.span,
        "eqeqeq",
        message,
        hint,
      )
    }
    swc_ecmascript::visit::visit_bin_expr(self, bin_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn eqeqeq_valid() {
    assert_lint_ok! {
      Eqeqeq,
      "midori === sapphire",
      "midori !== hazuki",
      "kumiko === null",
      "reina !== null",
      "null === null",
      "null !== null",
    };
  }

  #[test]
  fn eqeqeq_invalid() {
    assert_lint_err::<Eqeqeq>("a == b", 0);
    assert_lint_err::<Eqeqeq>("a != b", 0);
    assert_lint_err::<Eqeqeq>("typeof a == 'number'", 0);
    assert_lint_err::<Eqeqeq>("'string' != typeof a", 0);
    assert_lint_err::<Eqeqeq>("true == true", 0);
    assert_lint_err::<Eqeqeq>("2 == 3", 0);
    assert_lint_err::<Eqeqeq>("'hello' != 'world'", 0);
    assert_lint_err::<Eqeqeq>("a == null", 0);
    assert_lint_err::<Eqeqeq>("null != a", 0);
    assert_lint_err::<Eqeqeq>("true == null", 0);
    assert_lint_err::<Eqeqeq>("true != null", 0);
    assert_lint_err::<Eqeqeq>("null == null", 0);
    assert_lint_err::<Eqeqeq>("null != null", 0);
    assert_lint_err_on_line::<Eqeqeq>(
      r#"
a
==
b"#,
      2,
      0,
    );
    assert_lint_err::<Eqeqeq>("(a) == b", 0);
    assert_lint_err::<Eqeqeq>("(a) != b", 0);
    assert_lint_err::<Eqeqeq>("a == (b)", 0);
    assert_lint_err::<Eqeqeq>("a != (b)", 0);
    assert_lint_err::<Eqeqeq>("(a) == (b)", 0);
    assert_lint_err::<Eqeqeq>("(a) != (b)", 0);
    assert_lint_err_n::<Eqeqeq>("(a == b) == (c)", vec![0, 1]);
    assert_lint_err_n::<Eqeqeq>("(a != b) != (c)", vec![0, 1]);
    assert_lint_err::<Eqeqeq>("(a == b) === (c)", 1);
    assert_lint_err::<Eqeqeq>("(a == b) !== (c)", 1);
    assert_lint_err::<Eqeqeq>("(a === b) == (c)", 0);
    assert_lint_err::<Eqeqeq>("(a === b) != (c)", 0);
    assert_lint_err::<Eqeqeq>("a == b;", 0);
    assert_lint_err::<Eqeqeq>("a!=b;", 0);
    assert_lint_err::<Eqeqeq>("(a + b) == c;", 0);
    assert_lint_err::<Eqeqeq>("(a + b)  !=  c;", 0);
    assert_lint_err::<Eqeqeq>("((1) )  ==  (2);", 0);
  }
}
