// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use derive_more::Display;
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};
use swc_common::{Span, Spanned};

pub struct NoUnsafeFinally;

const CODE: &str = "no-unsafe-finally";
const HINT: &str = "Use of the control flow statements (`return`, `throw`, `break` and `continue`) in a `finally` block\
will most likely lead to undesired behavior.";

#[derive(Display)]
enum NoUnsafeFinallyMessage {
  #[display(fmt = "Unsafe usage of break statement")]
  Break,
  #[display(fmt = "Unsafe usage of continue statement")]
  Continue,
  #[display(fmt = "Unsafe usage of return statement")]
  Return,
  #[display(fmt = "Unsafe usage of throw statement")]
  Throw,
}

impl From<StmtKind<'_>> for NoUnsafeFinallyMessage {
  fn from(kind: StmtKind) -> Self {
    match kind {
      StmtKind::Break(_) => Self::Break,
      StmtKind::Continue(_) => Self::Continue,
      StmtKind::Return => Self::Return,
      StmtKind::Throw => Self::Throw,
    }
  }
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
    let kind = StmtKind::Break(break_stmt.label);
    if stmt_inside_finally(break_stmt.span(), kind, break_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, break_stmt.span(), kind);
    }
  }

  fn continue_stmt(
    &self,
    continue_stmt: &AstView::ContinueStmt,
    ctx: &mut Context,
  ) {
    let kind = StmtKind::Continue(continue_stmt.label);
    if stmt_inside_finally(
      continue_stmt.span(),
      kind,
      continue_stmt.into_node(),
    ) {
      add_diagnostic_with_hint(ctx, continue_stmt.span(), kind);
    }
  }

  fn return_stmt(&self, return_stmt: &AstView::ReturnStmt, ctx: &mut Context) {
    let kind = StmtKind::Return;
    if stmt_inside_finally(return_stmt.span(), kind, return_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, return_stmt.span(), kind);
    }
  }

  fn throw_stmt(&self, throw_stmt: &AstView::ThrowStmt, ctx: &mut Context) {
    let kind = StmtKind::Throw;
    if stmt_inside_finally(throw_stmt.span(), kind, throw_stmt.into_node()) {
      add_diagnostic_with_hint(ctx, throw_stmt.span(), kind);
    }
  }
}

#[derive(Clone, Copy)]
enum StmtKind<'a> {
  Break(Option<&'a AstView::Ident<'a>>),
  Continue(Option<&'a AstView::Ident<'a>>),
  Return,
  Throw,
}

impl<'a> StmtKind<'a> {
  fn is_break(&self) -> bool {
    matches!(self, StmtKind::Break(_))
  }

  fn is_continue(&self) -> bool {
    matches!(self, StmtKind::Continue(_))
  }

  fn label(&self) -> Option<&'a AstView::Ident<'a>> {
    if let StmtKind::Break(label) | StmtKind::Continue(label) = self {
      *label
    } else {
      None
    }
  }
}

/// Checks if the given span is contained in a `finally` block
fn stmt_inside_finally(
  stmt_span: Span,
  stmt_kind: StmtKind,
  cur_node: AstView::Node,
) -> bool {
  use AstView::Node::*;
  match (cur_node, stmt_kind.label()) {
    (Function(_), _) | (ArrowExpr(_), _) => false,
    (LabeledStmt(labeled_stmt), Some(label))
      if labeled_stmt.label.sym() == label.sym() =>
    {
      false
    }
    (SwitchStmt(_), None) if stmt_kind.is_break() => false,
    (ForStmt(_), None)
    | (ForOfStmt(_), None)
    | (ForInStmt(_), None)
    | (WhileStmt(_), None)
    | (DoWhileStmt(_), None)
      if (stmt_kind.is_break() || stmt_kind.is_continue()) =>
    {
      false
    }
    (
      TryStmt(AstView::TryStmt {
        finalizer: Some(ref f),
        ..
      }),
      _,
    ) if f.span().contains(stmt_span) => true,
    _ => {
      if let Some(parent) = cur_node.parent() {
        stmt_inside_finally(stmt_span, stmt_kind, parent)
      } else {
        false
      }
    }
  }
}

fn add_diagnostic_with_hint(
  ctx: &mut Context,
  span: Span,
  stmt_kind: StmtKind,
) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoUnsafeFinallyMessage::from(stmt_kind),
    HINT,
  );
}

#[cfg(test)]
mod tests {
  use super::*;

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
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    function bar() {
      return "hola!";
    }
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
    const f = (x) => {
      return x + 1;
    };
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
    class Foo {
      method(x: number): number {
        return x * 2;
      }
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
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  while (true) break;
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  while (true) continue;
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  do {
    break;
  } while (true)
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  do {
    continue;
  } while (true)
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  label: while (true) {
    if (x) break label;
    else continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (let i = 0; i < 100; i++) {
    break;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (let i = 0; i < 100; i++) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x of xs) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x of xs) {
    break;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x in xs) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x in xs) {
    break;
  }
}
      "#,
      r#"
      "#,
      r#"
      "#,
    };
  }

  #[test]
  fn no_unsafe_finally_invalid() {
    assert_lint_err! {
      NoUnsafeFinally,
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
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
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
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
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
          "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        }
      ],
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
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Throw,
          hint: HINT,
        }
      ],
      r#"
try {}
finally {
  try {}
  finally {
    throw new Error;
  }
}
     "#: [
        {
          line: 6,
          col: 4,
          message: NoUnsafeFinallyMessage::Throw,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  try {}
  finally {
    if (x) {
      return 0;
    } else {
      return 1;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        },
        {
          line: 8,
          col: 6,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        },
      ],
      r#"
function foo() {
  try {}
  finally {
    return () => {
      return 0;
    };
  }
}
     "#: [
        {
          line: 5,
          col: 4,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  label: try {
    return 0;
  } finally {
    break label;
  }
}
     "#: [
        {
          line: 6,
          col: 4,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      break;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      continue;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  switch (x) {
    case 0:
      try {}
      finally {
        break;
      }
  }
}
     "#: [
        {
          line: 7,
          col: 8,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  a: while (x) {
    try {}
    finally {
      switch (y) {
        case 0:
          break a;
      }
    }
  }
}
     "#: [
        {
          line: 8,
          col: 10,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      switch (y) {
        case 0:
          continue;
      }
    }
  }
}
     "#: [
        {
          line: 8,
          col: 10,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  a: switch (x) {
    case 0:
      try {}
      finally {
        switch (y) {
          case 1:
            break a;
        }
      }
  }
}
     "#: [
        {
          line: 9,
          col: 12,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
    };
  }
}
