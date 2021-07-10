// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::{Expr, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoNewSymbol;

const CODE: &str = "no-new-symbol";
const MESSAGE: &str = "`Symbol` cannot be called as a constructor.";

impl LintRule for NoNewSymbol {
  fn new() -> Box<Self> {
    Box::new(NoNewSymbol)
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
    let mut visitor = NoNewSymbolVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of `new` operators with built-in `Symbol`s

`Symbol`s are created by being called as a function, but we sometimes call it
with the `new` operator by mistake. This rule detects such wrong usage of the
`new` operator.

### Invalid:

```typescript
const foo = new Symbol("foo");
```

### Valid:

```typescript
const foo = Symbol("foo");

function func(Symbol: typeof SomeClass) {
  // This `Symbol` is not built-in one
  const bar = new Symbol();
}
```
"#
  }
}

struct NoNewSymbolVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoNewSymbolVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoNewSymbolVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if ident.sym == *"Symbol" {
        self.context.add_diagnostic(new_expr.span, CODE, MESSAGE);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_new_symbol_valid() {
    assert_lint_ok! {
      NoNewSymbol,
      "new Class()",
      "Symbol()",
    };
  }

  #[test]
  fn no_new_symbol_invalid() {
    assert_lint_err! {
      NoNewSymbol,
      "new Symbol()": [{ col: 0, message: MESSAGE }],
      // nested
      "new class { foo() { new Symbol(); } }": [{ col: 20, message: MESSAGE }],
    };
  }
}
