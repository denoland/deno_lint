// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::UnaryExpr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoDeleteVar;

const CODE: &str = "no-delete-var";

#[derive(Display)]
enum NoDeleteVarMessage {
  #[display(fmt = "Variables shouldn't be deleted")]
  Unexpected,
}

#[derive(Display)]
enum NoDeleteVarHint {
  #[display(fmt = "Remove the deletion statement")]
  Remove,
}

impl LintRule for NoDeleteVar {
  fn new() -> Box<Self> {
    Box::new(NoDeleteVar)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoDeleteVarVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the deletion of variables

`delete` is used to remove a property from an object.  Variables declared via
`var`, `let` and `const` cannot be deleted (`delete` will return false).  Setting
`strict` mode on will raise a syntax error when attempting to delete a variable.
    
### Invalid:
```typescript
const a = 1;
let b = 2;
var c = 3;
delete a; // would return false
delete b; // would return false
delete c; // would return false
```

### Valid:
```typescript
var obj = {
  a: 1,
};
delete obj.a; // returns true;
```
"#
  }
}

struct NoDeleteVarVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDeleteVarVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoDeleteVarVisitor<'c> {
  noop_visit_type!();

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _parent: &dyn Node) {
    if unary_expr.op != UnaryOp::Delete {
      return;
    }

    if let Expr::Ident(_) = *unary_expr.arg {
      self.context.add_diagnostic_with_hint(
        unary_expr.span,
        CODE,
        NoDeleteVarMessage::Unexpected,
        NoDeleteVarHint::Remove,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_delete_var_invalid() {
    assert_lint_err! {
      NoDeleteVar,
      r#"var someVar = "someVar"; delete someVar;"#: [
        {
          col: 25,
          message: variant!(NoDeleteVarMessage, Unexpected),
          hint: variant!(NoDeleteVarHint, Remove),
        }
      ],
    }
  }
}
