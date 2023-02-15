// Copyright 2020-2022 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::SourceRanged;
use std::sync::Arc;

#[derive(Debug)]
pub struct GuardForIn;

const CODE: &str = "guard-for-in";
const MESSAGE: &str = "Require `for-in` loops to include an `if` statement";
const HINT: &str = "The body of a `for-in` should be wrapped in an `if` statement to filter unwanted properties from the prototype.";

impl LintRule for GuardForIn {
  fn new() -> Arc<Self> {
    Arc::new(GuardForIn)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    GuardForInHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/guard_for_in.md")
  }
}

struct GuardForInHandler;

impl Handler for GuardForInHandler {
  fn for_in_stmt(
    &mut self,
    for_in_stmt: &deno_ast::view::ForInStmt,
    ctx: &mut Context,
  ) {
    use deno_ast::view::Stmt::{Block, Continue, Empty, If};

    match for_in_stmt.body {
      Empty(_) | If(_) => (),
      Block(block_stmt) => {
        match block_stmt.stmts[..] {
          // empty block
          [] => (),

          // block statement with only an if statement
          [stmt] => {
            if !matches!(stmt, If(_)) {
              ctx.add_diagnostic_with_hint(
                for_in_stmt.range(),
                CODE,
                MESSAGE,
                HINT,
              );
            }
          }

          // block statement that start with an if statement with only a continue statement
          [first, ..] => {
            let If(if_stmt) = first else {
              ctx.add_diagnostic_with_hint(
                for_in_stmt.range(),
                CODE,
                MESSAGE,
                HINT,
              );
              return;
            };

            match if_stmt.cons {
              Continue(_) => (),
              Block(inner_block_stmt) => {
                if !matches!(inner_block_stmt.stmts[..], [Continue(_)]) {
                  ctx.add_diagnostic_with_hint(
                    for_in_stmt.range(),
                    CODE,
                    MESSAGE,
                    HINT,
                  );
                }
              }
              _ => {
                ctx.add_diagnostic_with_hint(
                  for_in_stmt.range(),
                  CODE,
                  MESSAGE,
                  HINT,
                );
              }
            }
          }
        }
      }
      _ => {
        ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
      }
    };
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn guard_for_in_valid() {
    assert_lint_ok! {
      GuardForIn,
      r#"for (key in obj);"#,
      r#"
for (key in obj)
  if (Object.hasOwn(obj, key)) {}
"#,
      r#"
for (key in obj) {
  if (Object.hasOwn(obj, key)) {}
}
"#,
      r#"
for (key in obj) {
  if (!Object.hasOwn(obj, key)) continue;
}
"#,
      r#"
for (key in obj) {
  if (!Object.hasOwn(obj, key)) continue;
  foo(obj, key);
}
"#,
      r#"
for (key in obj) {
  if (!Object.hasOwn(obj, key)) {
    continue;
  }
}
"#,
      r#"
for (key in obj) {
  if (!Object.hasOwn(obj, key)) {
    continue;
  }
  foo(obj, key);
}
"#,
    };
  }

  #[test]
  fn guard_for_in_invalid() {
    assert_lint_err! {
      GuardForIn,
      MESSAGE,
      HINT,
      r#"
for (key in obj)
  foo(obj, key);
"#: [{ line: 2, col: 0 }],
      r#"
for (key in obj) {
  foo(obj, key);
}
"#: [{ line: 2, col: 0 }],
      r#"
for (key in obj) {
  foo(obj, key);
  bar(obj, key);
}
"#: [{ line: 2, col: 0 }],
      r#"
for (key in obj) {
  if (!Object.hasOwn(obj, key)) {
    foo(obj, key);
    continue;
  }
  bar(obj, key);
}
"#: [{ line: 2, col: 0 }],
    };
  }
}
