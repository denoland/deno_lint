// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use derive_more::Display;
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};
use swc_common::{Span, Spanned};

pub struct NoUnsafeFinally;

const CODE: &str = "no-unsafe-finally";
const HINT: &str = "Use of the control flow statements (`return`, `throw`, `break` and `continue`) in a `finally` block will most likely lead to undesired behavior. It's recommended to take a second look.";

#[derive(Display)]
enum NoUnsafeFinallyMessage {
  #[display(fmt = "Unsafe usage of {} statement", _0)]
  UnsafeUsage(String),
}

impl LintRule for NoUnsafeFinally {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeFinally)
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
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    NoUnsafeFinallyHandler.traverse(program, context);
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

struct NoUnsafeFinallyHandler;

impl Handler for NoUnsafeFinallyHandler {
  fn break_stmt(&self, break_stmt: &AstView::BreakStmt, ctx: &mut Context) {
    if stmt_inside_finally(break_stmt.span(), true, break_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, break_stmt.span(), "break");
    }
  }

  fn continue_stmt(
    &self,
    continue_stmt: &AstView::ContinueStmt,
    ctx: &mut Context,
  ) {
    if stmt_inside_finally(
      continue_stmt.span(),
      false,
      continue_stmt.into_node(),
    ) {
      add_diagnostic_with_hint(ctx, continue_stmt.span(), "continue");
    }
  }

  fn return_stmt(&self, return_stmt: &AstView::ReturnStmt, ctx: &mut Context) {
    if stmt_inside_finally(return_stmt.span(), false, return_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, return_stmt.span(), "return");
    }
  }

  fn throw_stmt(&self, throw_stmt: &AstView::ThrowStmt, ctx: &mut Context) {
    if stmt_inside_finally(throw_stmt.span(), false, throw_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, throw_stmt.span(), "throw");
    }
  }
}

/// Checks if the given span is contained in a `finally` block
fn stmt_inside_finally(
  stmt_span: Span,
  is_break_stmt: bool,
  cur_node: AstView::Node,
) -> bool {
  use AstView::Node::*;
  match cur_node {
    FnDecl(_) | FnExpr(_) | ArrowExpr(_) => false,
    SwitchStmt(_) if is_break_stmt => false,
    TryStmt(AstView::TryStmt {
      finalizer: Some(ref f),
      ..
    }) if f.span().contains(stmt_span) => true,
    _ => {
      if let Some(parent) = cur_node.parent() {
        stmt_inside_finally(stmt_span, is_break_stmt, parent)
      } else {
        false
      }
    }
  }
}

fn add_diagnostic_with_hint(ctx: &mut Context, span: Span, stmt_type: &str) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoUnsafeFinallyMessage::UnsafeUsage(stmt_type.to_string()),
    HINT,
  );
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
      8,
      4,
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
      8,
      4,
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
      8,
      4,
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
      8,
      4,
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
      6,
      4,
    );
  }
}
