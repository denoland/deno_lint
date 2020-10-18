// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_common::Spanned;
use swc_ecmascript::ast::BinaryOp::{EqEq, EqEqEq, NotEq, NotEqEq};
use swc_ecmascript::ast::Expr::{Lit, Unary};
use swc_ecmascript::ast::Lit::Str;
use swc_ecmascript::ast::UnaryOp::TypeOf;
use swc_ecmascript::ast::{BinExpr, Module};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

pub struct ValidTypeof;

impl LintRule for ValidTypeof {
  fn new() -> Box<Self> {
    Box::new(ValidTypeof)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "valid-typeof"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = ValidTypeofVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn docs(&self) -> &'static str {
    r#"Restricts the use of the `typeof` operator to a specific set of string literals.

When used with a value the `typeof` operator returns one of the following strings:
- `"undefined"`
- `"object"`
- `"boolean"`
- `"number"`
- `"string"`
- `"function"`
- `"symbol"`
- `"bigint"`

This rule disallows comparison with anything other than one of these string literals when using the `typeof` operator, as this likely represents a typing mistake in the string. The rule also disallows comparing the result of a `typeof` operation with any non-string literal value, such as `undefined`, which can represent an inadvertent use of a keyword instead of a string. This includes comparing against string variables even if they contain one of the above values as this cannot be guaranteed. An exception to this is comparing the results of two `typeof` operations as these are both guaranteed to return on of the above strings.

### Invalid:
```typescript
typeof foo === "strnig"
```
```typescript
typeof foo == "undefimed"
```
```typescript
typeof bar != "nunber"
```
```typescript
typeof bar !== "fucntion"
```
```typescript
typeof foo === undefined
```
```typescript
typeof bar == Object
```
```typescript
typeof baz === anotherVariable
```
```typescript
typeof foo == 5
```

### Valid:
```typescript
typeof foo === "undefined"
```
```typescript
typeof bar == "object"
```
```typescript
typeof baz === "string"
```
```typescript
typeof bar === typeof qux
```"#
  }
}

struct ValidTypeofVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ValidTypeofVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for ValidTypeofVisitor<'c> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    if !bin_expr.is_eq_expr() {
      return;
    }

    match (&*bin_expr.left, &*bin_expr.right) {
      (Unary(unary), operand) | (operand, Unary(unary))
        if unary.op == TypeOf =>
      {
        match operand {
          Unary(unary) if unary.op == TypeOf => {}
          Lit(Str(str)) => {
            if !is_valid_typeof_string(&str.value) {
              self.context.add_diagnostic(
                str.span,
                "valid-typeof",
                "Invalid typeof comparison value",
              );
            }
          }
          _ => {
            self.context.add_diagnostic(
              operand.span(),
              "valid-typeof",
              "Invalid typeof comparison value",
            );
          }
        }
      }
      _ => {}
    }
  }
}

fn is_valid_typeof_string(str: &str) -> bool {
  matches!(
    str,
    "undefined"
      | "object"
      | "boolean"
      | "number"
      | "string"
      | "function"
      | "symbol"
      | "bigint"
  )
}

trait EqExpr {
  fn is_eq_expr(&self) -> bool;
}

impl EqExpr for BinExpr {
  fn is_eq_expr(&self) -> bool {
    matches!(self.op, EqEq | NotEq | EqEqEq | NotEqEq)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn valid_typeof_valid() {
    assert_lint_ok_macro! {
      ValidTypeof,
      r#"
typeof foo === "string"
typeof bar == "undefined"
      "#,
      r#"typeof bar === typeof qux"#,
    };
  }

  #[test]
  fn valid_typeof_invalid() {
    assert_lint_err::<ValidTypeof>(r#"typeof foo === "strnig""#, 15);
    assert_lint_err::<ValidTypeof>(r#"typeof foo == "undefimed""#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof bar != "nunber""#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof bar !== "fucntion""#, 15);
    assert_lint_err::<ValidTypeof>(r#"typeof foo === undefined"#, 15);
    assert_lint_err::<ValidTypeof>(r#"typeof bar == Object"#, 14);
    assert_lint_err::<ValidTypeof>(r#"typeof baz === anotherVariable"#, 15);
  }
}
