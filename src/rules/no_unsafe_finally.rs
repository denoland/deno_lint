// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::{view as ast_view, SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
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
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoUnsafeFinallyHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unsafe_finally.md")
  }
}

struct NoUnsafeFinallyHandler;

impl Handler for NoUnsafeFinallyHandler {
  fn break_stmt(
    &mut self,
    break_stmt: &ast_view::BreakStmt,
    ctx: &mut Context,
  ) {
    let kind = StmtKind::Break(break_stmt.label);
    if stmt_inside_finally(break_stmt.range(), kind, break_stmt.as_node()) {
      add_diagnostic_with_hint(ctx, break_stmt.range(), kind);
    }
  }

  fn continue_stmt(
    &mut self,
    continue_stmt: &ast_view::ContinueStmt,
    ctx: &mut Context,
  ) {
    let kind = StmtKind::Continue(continue_stmt.label);
    if stmt_inside_finally(continue_stmt.range(), kind, continue_stmt.as_node())
    {
      add_diagnostic_with_hint(ctx, continue_stmt.range(), kind);
    }
  }

  fn return_stmt(
    &mut self,
    return_stmt: &ast_view::ReturnStmt,
    ctx: &mut Context,
  ) {
    let kind = StmtKind::Return;
    if stmt_inside_finally(return_stmt.range(), kind, return_stmt.as_node()) {
      add_diagnostic_with_hint(ctx, return_stmt.range(), kind);
    }
  }

  fn throw_stmt(
    &mut self,
    throw_stmt: &ast_view::ThrowStmt,
    ctx: &mut Context,
  ) {
    let kind = StmtKind::Throw;
    if stmt_inside_finally(throw_stmt.range(), kind, throw_stmt.as_node()) {
      add_diagnostic_with_hint(ctx, throw_stmt.range(), kind);
    }
  }
}

#[derive(Clone, Copy)]
enum StmtKind<'a> {
  Break(Option<&'a ast_view::Ident<'a>>),
  Continue(Option<&'a ast_view::Ident<'a>>),
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

  fn label(&self) -> Option<&'a ast_view::Ident<'a>> {
    if let StmtKind::Break(label) | StmtKind::Continue(label) = self {
      *label
    } else {
      None
    }
  }
}

/// Checks if the given range is contained in a `finally` block
fn stmt_inside_finally(
  stmt_range: SourceRange,
  stmt_kind: StmtKind,
  cur_node: ast_view::Node,
) -> bool {
  use deno_ast::view::Node::*;
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
      TryStmt(ast_view::TryStmt {
        finalizer: Some(ref f),
        ..
      }),
      _,
    ) if f.range().contains(&stmt_range) => true,
    _ => {
      if let Some(parent) = cur_node.parent() {
        stmt_inside_finally(stmt_range, stmt_kind, parent)
      } else {
        false
      }
    }
  }
}

fn add_diagnostic_with_hint(
  ctx: &mut Context,
  range: SourceRange,
  stmt_kind: StmtKind,
) {
  ctx.add_diagnostic_with_hint(
    range,
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
