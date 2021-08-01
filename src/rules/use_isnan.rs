// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct UseIsNaN;

const CODE: &str = "use-isnan";

#[derive(Display)]
enum UseIsNaNMessage {
  #[display(fmt = "Use the isNaN function to compare with NaN")]
  Comparison,

  #[display(
    fmt = "'switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch"
  )]
  SwitchUnmatched,

  #[display(
    fmt = "'case NaN' can never match. Use Number.isNaN before the switch"
  )]
  CaseUnmatched,
}

impl LintRule for UseIsNaN {
  fn new() -> Box<Self> {
    Box::new(UseIsNaN)
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
    let mut visitor = UseIsNaNVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows comparisons to `NaN`.

Because `NaN` is unique in JavaScript by not being equal to anything, including itself, the results of comparisons to `NaN` are confusing:

- `NaN === NaN` or `NaN == NaN` evaluate to `false`
- `NaN !== NaN` or `NaN != NaN` evaluate to `true`

Therefore, this rule makes you use the `isNaN()` or `Number.isNaN()` to judge the value is `NaN` or not.

### Invalid:

```typescript
if (foo == NaN) {
  // ...
}

if (foo != NaN) {
  // ...
}

switch (NaN) {
  case foo:
    // ...
}

switch (foo) {
  case NaN:
    // ...
}
```

### Valid:

```typescript
if (isNaN(foo)) {
  // ...
}

if (!isNaN(foo)) {
  // ...
}
```
"#
  }
}

struct UseIsNaNVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> UseIsNaNVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

fn is_nan_identifier(ident: &swc_ecmascript::ast::Ident) -> bool {
  ident.sym == *"NaN"
}

impl<'c, 'view> Visit for UseIsNaNVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_bin_expr(
    &mut self,
    bin_expr: &swc_ecmascript::ast::BinExpr,
    _parent: &dyn Node,
  ) {
    if bin_expr.op == swc_ecmascript::ast::BinaryOp::EqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::NotEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::EqEqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::NotEqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::Lt
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::LtEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::Gt
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::GtEq
    {
      if let swc_ecmascript::ast::Expr::Ident(ident) = &*bin_expr.left {
        if is_nan_identifier(ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            CODE,
            UseIsNaNMessage::Comparison,
          );
        }
      }
      if let swc_ecmascript::ast::Expr::Ident(ident) = &*bin_expr.right {
        if is_nan_identifier(ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            CODE,
            UseIsNaNMessage::Comparison,
          );
        }
      }
    }
  }

  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecmascript::ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    if let swc_ecmascript::ast::Expr::Ident(ident) = &*switch_stmt.discriminant
    {
      if is_nan_identifier(ident) {
        self.context.add_diagnostic(
          switch_stmt.span,
          CODE,
          UseIsNaNMessage::SwitchUnmatched,
        );
      }
    }

    for case in &switch_stmt.cases {
      if let Some(expr) = &case.test {
        if let swc_ecmascript::ast::Expr::Ident(ident) = &**expr {
          if is_nan_identifier(ident) {
            self.context.add_diagnostic(
              case.span,
              CODE,
              UseIsNaNMessage::CaseUnmatched,
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn use_isnan_invalid() {
    assert_lint_err! {
      UseIsNaN,
      "42 === NaN": [
      {
        col: 0,
        message: UseIsNaNMessage::Comparison,
      }],
      r#"
switch (NaN) {
  case NaN:
    break;
  default:
    break;
}
        "#: [
      {
        line: 2,
        col: 0,
        message: UseIsNaNMessage::SwitchUnmatched,
      },
      {
        line: 3,
        col: 2,
        message: UseIsNaNMessage::CaseUnmatched,
      }],
    }
  }
}
