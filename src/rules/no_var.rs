// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::VarDecl;
use swc_ecmascript::ast::VarDeclKind;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoVar;

const MESSAGE: &str = "`var` keyword is not allowed.";

impl LintRule for NoVar {
  fn new() -> Box<Self> {
    Box::new(NoVar)
  }

  fn code(&self) -> &'static str {
    "no-var"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoVarVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Enforces the use of block scoped variables over more error prone function scoped variables. Block scoped variables are defined using `const` and `let` keywords.

`const` and `let` keywords ensure the variables defined using these keywords are not accessible outside their block scope. On the other hand, variables defined using `var` keyword are only limited by their function scope.

### Invalid:
```typescript
var foo = 'bar';
```

### Valid:
```typescript
const foo = 1;
let bar = 2;
```
"#
  }
}

struct NoVarVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoVarVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoVarVisitor<'c> {
  noop_visit_type!();

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.kind == VarDeclKind::Var {
      self
        .context
        .add_diagnostic(var_decl.span, "no-var", MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_var_invalid() {
    assert_lint_err::<NoVar>(
      r#"var someVar = "someString"; const c = "c"; let a = "a";"#,
      0,
    );

    assert_lint_err!(
      NoVar,
      "var foo = 'bar';": [{
        col: 0,
        message: MESSAGE,
      }],
      "let foo = 'bar'; var i = 0;": [{
        col: 17,
        message: MESSAGE,
      }],
      "let foo = 'bar'; var i = 0; var x = 1;": [
        {
          col: 17,
          message: MESSAGE,
        },
        {
          col: 28,
          message: MESSAGE,
        }
      ]
    );
  }

  #[test]
  fn no_var_valid() {
    assert_lint_ok::<NoVar>(r#"let foo = 0; const bar = "bar""#);
  }
}
