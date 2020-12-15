// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_common::Spanned;
use swc_ecmascript::ast::{
  BinExpr, BinaryOp, Expr, Ident, Lit, Program, UnaryOp,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

pub struct ValidTypeof;

const CODE: &str = "valid-typeof";
const MESSAGE: &str = "Invalid typeof comparison value";

impl LintRule for ValidTypeof {
  fn new() -> Box<Self> {
    Box::new(ValidTypeof)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = ValidTypeofVisitor::new(context);
    visitor.visit_program(program, program);
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

This rule disallows comparison with anything other than one of these string literals when using the `typeof` operator, as this likely represents a typing mistake in the string. The rule also disallows comparing the result of a `typeof` operation with any non-string literal value i.e. `Object`, `undefined` or `null`, which can represent an inadvertent use of a keyword instead of a string.

### Invalid:
```typescript
typeof foo === "strnig"
```
```typescript
typeof foo == "undefimed"
```
```typescript
typeof foo != "nunber"
```
```typescript
typeof foo !== `fucntion`
```
```typescript
typeof foo == Object
```
```typescript
typeof foo === undefined
```
```typescript
typeof foo === null
```
```typescript
typeof foo == 5
```

### Valid:
```typescript
typeof foo === "undefined"
```
```typescript
typeof foo == "object"
```
```typescript
typeof foo !== "string"
```
```typescript
typeof foo != typeof qux
```
```typescript
typeof foo === anotherVariable
```
```typescript
typeof foo === `bigint`
```
```typescript
typeof foo === `object${bar}`
```
"#
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
      (Expr::Unary(unary), operand) | (operand, Expr::Unary(unary))
        if unary.op == UnaryOp::TypeOf =>
      {
        match operand {
          Expr::Unary(unary) if unary.op == UnaryOp::TypeOf => {}
          Expr::Ident(ident) if is_valid_ident(ident) => {}
          Expr::Tpl(tpl) if !tpl.exprs.is_empty() => {}
          Expr::Lit(Lit::Str(str)) => {
            if !is_valid_typeof_string(&str.value) {
              self.context.add_diagnostic(str.span, CODE, MESSAGE);
            }
          }
          _ => {
            self.context.add_diagnostic(operand.span(), CODE, MESSAGE);
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

fn is_valid_ident(ident: &Ident) -> bool {
  !matches!(ident.as_ref(), "Object" | "undefined" | "null")
}

trait EqExpr {
  fn is_eq_expr(&self) -> bool;
}

impl EqExpr for BinExpr {
  fn is_eq_expr(&self) -> bool {
    use BinaryOp::*;
    matches!(self.op, EqEq | NotEq | EqEqEq | NotEqEq)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_typeof_valid() {
    assert_lint_ok! {
      ValidTypeof,
        "typeof foo === 'string'",
        "typeof foo === 'object'",
        "typeof foo === 'function'",
        "typeof foo === 'undefined'",
        "typeof foo === 'boolean'",
        "typeof foo === 'number'",
        "typeof foo === 'bigint'",
        "'string' === typeof foo",
        "'object' === typeof foo",
        "'function' === typeof foo",
        "'undefined' === typeof foo",
        "'boolean' === typeof foo",
        "'number' === typeof foo",
        "typeof foo === typeof bar",
        "typeof foo === baz",
        "typeof foo !== someType",
        "typeof bar != someType",
        "someType === typeof bar",
        "someType == typeof bar",
        "typeof foo == 'string'",
        "typeof(foo) === 'string'",
        "typeof(foo) !== 'string'",
        "typeof(foo) == 'string'",
        "typeof(foo) != 'string'",
        "var oddUse = typeof foo + 'thing'",
        "typeof foo === `str${somethingElse}`",
    };
  }

  #[test]
  fn valid_typeof_invalid() {
    assert_lint_err! {
      ValidTypeof,
      "typeof foo === 'strnig'": [{
        col: 15,
        message: MESSAGE
      }],
      "'strnig' === typeof foo": [{
        col: 0,
        message: MESSAGE
      }],
      "typeof foo !== 'strnig'": [{
        col: 15,
        message: MESSAGE
      }],
      "'strnig' !== typeof foo": [{
        col: 0,
        message: MESSAGE
      }],
      "typeof foo == 'undefimed'": [{
        col: 14,
        message: MESSAGE
      }],
      "typeof foo != 'undefimed'": [{
        col: 14,
        message: MESSAGE
      }],
      "if (typeof foo === 'undefimed') {}": [{
        col: 19,
        message: MESSAGE
      }],
      "if (typeof foo !== 'undefimed') {}": [{
        col: 19,
        message: MESSAGE
      }],
      "if ('undefimed' === typeof foo) {}": [{
        col: 4,
        message: MESSAGE
      }],
      "if ('undefimed' !== typeof foo) {}": [{
        col: 4,
        message: MESSAGE
      }],
      "if (typeof foo == 'undefimed') {}": [{
        col: 18,
        message: MESSAGE
      }],
      "if (typeof foo != 'undefimed') {}": [{
        col: 18,
        message: MESSAGE
      }],
      "if ('undefimed' == typeof foo) {}": [{
        col: 4,
        message: MESSAGE
      }],
      "if ('undefimed' != typeof foo) {}": [{
        col: 4,
        message: MESSAGE
      }],
      "typeof foo != 'nunber'": [{
        col: 14,
        message: MESSAGE
      }],
      "typeof foo !== 'fucntion'": [{
        col: 15,
        message: MESSAGE
      }],
      "typeof foo !== `bigitn`": [{
        col: 15,
        message: MESSAGE
      }],
      "typeof foo === Object": [{
        col: 15,
        message: MESSAGE
      }],
      "typeof foo === undefined": [{
        col: 15,
        message: MESSAGE
      }],
      "typeof foo === null": [{
        col: 15,
        message: MESSAGE
      }],
    }
  }
}
