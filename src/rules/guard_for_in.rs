use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
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

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
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
    ctx: &mut Context
  ) {
    use deno_ast::view::Stmt::{Block, Continue, Empty, If};

    match for_in_stmt.body {
      Empty(_) | If(_) => (),
      Block(block_stmt) => {
        match block_stmt.stmts.len() {
          // empty block
          0 => (),

          // block statement with only an if statement
          1 => {
            let If(_) = block_stmt.stmts.first().unwrap() else {
              ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
              return;
            };
          },

          // block statement that start with an if statement with only a continue statement
          _ => {
            let if_stmt =
              if let If(if_stmt) = block_stmt.stmts.first().unwrap() { if_stmt }
              else {
                ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
                return;
              };

            match if_stmt.cons {
              Continue(_) => (),
              Block(inner_block_stmt) => {
                if inner_block_stmt.stmts.len() != 1 {
                  ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
                  return;
                }
                let Continue(_) = inner_block_stmt.stmts.first().unwrap() else {
                  ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
                  return;
                };
              }
              _ => {
                ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
                return;
              }
            }
          }
        }
      },
      _ => {
        ctx.add_diagnostic_with_hint(for_in_stmt.range(), CODE, MESSAGE, HINT);
        return;
      },
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
    };
  }
}
