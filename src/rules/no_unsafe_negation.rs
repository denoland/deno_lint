// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use derive_more::Display;
use dprint_swc_ecma_ast_view as AstView;
use if_chain::if_chain;
use swc_common::Spanned;

pub struct NoUnsafeNegation;

const CODE: &str = "no-unsafe-negation";

#[derive(Display)]
enum NoUnsafeNegationMessage {
  #[display(fmt = "Unexpected negating the left operand of `{}` operator", _0)]
  Unexpected(String),
}

const HINT: &str = "Add parentheses to clarify which range the negation operator should be applied to";

impl LintRule for NoUnsafeNegation {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeNegation)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: AstView::Program,
  ) {
    NoUnsafeNegationHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the usage of negation operator `!` as the left operand of
relational operators.

`!` operators appearing in the left operand of the following operators will
sometimes cause an unexpected behavior because of the operator precedence: 

- `in` operator
- `instanceof` operator

For example, when developers write a code like `!key in someObject`, most
likely they want it to behave just like `!(key in someObject)`, but actually it
behaves like `(!key) in someObject`.
This lint rule warns such usage of `!` operator so it will be less confusing.

### Invalid:
```typescript
if (!key in object) {}
if (!foo instanceof Foo) {}
```

### Valid:
```typescript
if (!(key in object)) {}
if (!(foo instanceof Foo)) {}
if ((!key) in object) {}
if ((!foo) instanceof Foo) {}
```
"#
  }
}

struct NoUnsafeNegationHandler;

impl Handler for NoUnsafeNegationHandler {
  fn bin_expr(&mut self, bin_expr: &AstView::BinExpr, ctx: &mut Context) {
    use AstView::{BinaryOp, Expr, UnaryOp};
    if_chain! {
      if matches!(bin_expr.op(), BinaryOp::In | BinaryOp::InstanceOf);
      if let Expr::Unary(unary_expr) = &bin_expr.left;
      if unary_expr.op() == UnaryOp::Bang;
      then {
        ctx.add_diagnostic_with_hint(
          bin_expr.span(),
          CODE,
          NoUnsafeNegationMessage::Unexpected(bin_expr.op().to_string()),
          HINT,
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unsafe_negation_valid() {
    assert_lint_ok! {
      NoUnsafeNegation,
      "1 in [1, 2, 3]",
      "key in object",
      "foo instanceof Date",
      "!(1 in [1, 2, 3])",
      "!(key in object)",
      "!(foo instanceof Date)",
      "(!key) in object",
      "(!foo) instanceof Date",
    };
  }

  #[test]
  fn no_unsafe_negation_invalid() {
    assert_lint_err! {
      NoUnsafeNegation,
      "!1 in [1, 2, 3]": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "in"),
          hint: HINT
        }
      ],
      "!key in object": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "in"),
          hint: HINT
        }
      ],
      "!foo instanceof Date": [
        {
          col: 0,
          message: variant!(NoUnsafeNegationMessage, Unexpected, "instanceof"),
          hint: HINT
        }
      ],
    };
  }
}
