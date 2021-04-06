// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_common::Spanned;
use swc_ecmascript::ast::BinExpr;
use swc_ecmascript::ast::BinaryOp::{EqEq, EqEqEq, NotEq, NotEqEq};
use swc_ecmascript::ast::Expr::{Lit, Unary};
use swc_ecmascript::ast::Lit::Str;
use swc_ecmascript::ast::UnaryOp::TypeOf;
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

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = ValidTypeofVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
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

struct ValidTypeofVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> ValidTypeofVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for ValidTypeofVisitor<'c, 'view> {
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

  #[test]
  fn valid_typeof_valid() {
    assert_lint_ok! {
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
    assert_lint_err! {
      ValidTypeof,
      r#"typeof foo === "strnig""#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo == "undefimed""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar != "nunber""#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof bar !== "fucntion""#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof foo === undefined"#: [{
        col: 15,
        message: MESSAGE
      }],
      r#"typeof bar == Object"#: [{
        col: 14,
        message: MESSAGE
      }],
      r#"typeof baz === anotherVariable"#: [{
        col: 15,
        message: MESSAGE
      }],
    }
  }
}
