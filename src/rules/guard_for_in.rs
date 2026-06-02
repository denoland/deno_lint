// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct GuardForIn;

const CODE: &str = "guard-for-in";
const MESSAGE: &str = "Require `for-in` loops to include an `if` statement";
const HINT: &str = "The body of a `for-in` should be wrapped in an `if` statement to filter unwanted properties from the prototype.";

impl LintRule for GuardForIn {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = GuardForInHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct GuardForInHandler;

impl Handler<'_> for GuardForInHandler {
  fn for_in_statement(
    &mut self,
    for_in_stmt: &ForInStatement,
    ctx: &mut Context,
  ) {
    use Statement::{
      BlockStatement, ContinueStatement, EmptyStatement, IfStatement,
    };

    match &for_in_stmt.body {
      EmptyStatement(_) | IfStatement(_) => (),
      BlockStatement(block_stmt) => {
        match block_stmt.body.as_slice() {
          // empty block
          [] => (),

          // block statement with only an if statement
          [stmt] => {
            if !matches!(stmt, IfStatement(_)) {
              ctx.add_diagnostic_with_hint(
                for_in_stmt.span,
                CODE,
                MESSAGE,
                HINT,
              );
            }
          }

          // block statement that start with an if statement with only a continue statement
          [first, ..] => {
            let IfStatement(if_stmt) = first else {
              ctx.add_diagnostic_with_hint(
                for_in_stmt.span,
                CODE,
                MESSAGE,
                HINT,
              );
              return;
            };

            match &if_stmt.consequent {
              ContinueStatement(_) => (),
              BlockStatement(inner_block_stmt) => {
                if !matches!(
                  inner_block_stmt.body.as_slice(),
                  [ContinueStatement(_)]
                ) {
                  ctx.add_diagnostic_with_hint(
                    for_in_stmt.span,
                    CODE,
                    MESSAGE,
                    HINT,
                  );
                }
              }
              _ => {
                ctx.add_diagnostic_with_hint(
                  for_in_stmt.span,
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
        ctx.add_diagnostic_with_hint(for_in_stmt.span, CODE, MESSAGE, HINT);
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
      r#"for (const key in obj);"#,
      r#"
for (const key in obj)
  if (Object.hasOwn(obj, key)) {}
"#,
      r#"
for (const key in obj) {
  if (Object.hasOwn(obj, key)) {}
}
"#,
      r#"
for (const key in obj) {
  if (!Object.hasOwn(obj, key)) continue;
}
"#,
      r#"
for (const key in obj) {
  if (!Object.hasOwn(obj, key)) continue;
  foo(obj, key);
}
"#,
      r#"
for (const key in obj) {
  if (!Object.hasOwn(obj, key)) {
    continue;
  }
}
"#,
      r#"
for (const key in obj) {
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
for (const key in obj)
  foo(obj, key);
"#: [{ line: 2, col: 0 }],
      r#"
for (const key in obj) {
  foo(obj, key);
}
"#: [{ line: 2, col: 0 }],
      r#"
for (const key in obj) {
  foo(obj, key);
  bar(obj, key);
}
"#: [{ line: 2, col: 0 }],
      r#"
for (const key in obj) {
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
