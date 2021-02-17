// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_common::Span;
use swc_ecmascript::ast::Stmt::{Break, Continue, Return, Throw};
use swc_ecmascript::ast::TryStmt;
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

pub struct NoUnsafeFinally;

impl LintRule for NoUnsafeFinally {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeFinally)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-unsafe-finally"
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoUnsafeFinallyVisitor::new(context);
    match program {
        ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
        ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of control flow statements within `finally` blocks.

Use of the control flow statements (`return`, `throw`, `break` and `continue`) overrides the usage of any control flow statements that might have been used in the `try` or `catch` blocks, which is usually not the desired behaviour.

### Invalid:
```typescript
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    return 3;
  }
};
```
```typescript
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    throw new Error;
  }
};
```
### Valid:
```typescript
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    console.log("hola!");
  }
};
```"#
  }
}

struct NoUnsafeFinallyVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoUnsafeFinallyVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, stmt_type: &str) {
    self.context.add_diagnostic(
      span,
      "no-unsafe-finally",
      format!("Unsafe usage of {}Statement", stmt_type),
    );
  }
}

impl<'c> VisitAll for NoUnsafeFinallyVisitor<'c> {
  noop_visit_type!();

  fn visit_try_stmt(&mut self, try_stmt: &TryStmt, _parent: &dyn Node) {
    if let Some(finally_block) = &try_stmt.finalizer {
      for stmt in &finally_block.stmts {
        match stmt {
          Break(_) => self.add_diagnostic(finally_block.span, "Break"),
          Continue(_) => self.add_diagnostic(finally_block.span, "Continue"),
          Return(_) => self.add_diagnostic(finally_block.span, "Return"),
          Throw(_) => self.add_diagnostic(finally_block.span, "Throw"),
          _ => {}
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unsafe_finally_valid() {
    assert_lint_ok! {
      NoUnsafeFinally,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    console.log("hola!");
  }
};
     "#,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    let a = function() {
      return "hola!";
    }
  }
};
     "#,
      r#"
let foo = function(a) {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    switch(a) {
      case 1: {
        console.log("hola!")
        break;
      }
    }
  }
};
      "#,
    };
  }

  #[test]
  fn no_unsafe_finally_invalid() {
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    break;
  }
};
     "#,
      7,
      12,
    );
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    continue;
  }
};
     "#,
      7,
      12,
    );
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    return 3;
  }
};
          "#,
      7,
      12,
    );
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    throw new Error;
  }
};
     "#,
      7,
      12,
    );
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
try {}
finally {
  try {}
  finally {
    throw new Error;
  }
}
     "#,
      5,
      10,
    );
  }
}
