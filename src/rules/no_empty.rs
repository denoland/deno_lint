// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{BlockStatement, Program, SwitchStatement};

#[derive(Debug)]
pub struct NoEmpty;

const CODE: &str = "no-empty";

impl LintRule for NoEmpty {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoEmptyHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoEmptyHandler;

fn block_contains_comments(block_stmt: &BlockStatement, ctx: &Context) -> bool {
  ctx.all_comments().any(|comment| {
    comment.span.start > block_stmt.span.start
      && comment.span.end < block_stmt.span.end
  })
}

impl Handler<'_> for NoEmptyHandler {
  fn block_statement(
    &mut self,
    block_stmt: &BlockStatement,
    ctx: &mut Context,
  ) {
    // In OXC, function/arrow/constructor bodies are FunctionBody, not
    // BlockStatement, so we don't need to check parents here.
    if block_stmt.body.is_empty() && !block_contains_comments(block_stmt, ctx) {
      ctx.add_diagnostic_with_hint(
        block_stmt.span,
        CODE,
        "Empty block statement",
        "Add code or comment to the empty block",
      );
    }
  }

  fn switch_statement(&mut self, switch: &SwitchStatement, ctx: &mut Context) {
    if switch.cases.is_empty() {
      ctx.add_diagnostic_with_hint(
        switch.span,
        CODE,
        "Empty switch statement",
        "Add case statement(s) to the empty switch, or remove",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_valid() {
    assert_lint_ok! {
      NoEmpty,
      r#"function foobar() {}"#,
      r#"
class Foo {
  constructor() {}
}
      "#,
      r#"if (foo) { var bar = ""; }"#,
      r#"
if (foo) {
  // This block is not empty
}
    "#,
      r#"
if (foo) {
  /* This block is not empty */
}
    "#,
      r#"
    switch (foo) {
      case bar:
        break;
    }
      "#,
      r#"
if (foo) {
  if (bar) {
    var baz = "";
  }
}
      "#,
      "const testFunction = (): void => {};",
      r#"
      switch (foo) {
        case 1:
        case 2:
          break;
        default:
          return 1;
      }
      "#,

      // https://github.com/denoland/deno_lint/issues/469
      "try { foo(); } catch { /* pass */ }",
      r#"
try {
  foo();
} catch { // pass
}
      "#,
    };
  }

  #[test]
  fn no_empty_invalid() {
    assert_lint_err! {
      NoEmpty,
      "if (foo) { }": [
        {
          col: 9,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      r#"
// This is an empty block
if (foo) { }
      "#: [
        {
          line: 3,
          col: 9,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "while (foo) { }": [
        {
          col: 12,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "do { } while (foo);": [
        {
          col: 3,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(;;) { }": [
        {
          col: 8,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(var foo in bar) { }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(var foo of bar) { }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "switch (foo) { }": [
        {
          col: 0,
          message: "Empty switch statement",
          hint: "Add case statement(s) to the empty switch, or remove",
        }
      ],
      "try { } catch (err) { }": [
        {
          col: 4,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "try { } catch (err) { } finally { }": [
        {
          col: 4,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 32,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { if (bar) { } }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { while (bar) { } }": [
        {
          col: 23,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { do { } while (bar); }": [
        {
          col: 14,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(;;) { } }": [
        {
          col: 19,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(var bar in foo) { } }": [
        {
          col: 31,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(var bar of foo) { } }": [
        {
          col: 31,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { switch (foo) { } }": [
        {
          col: 11,
          message: "Empty switch statement",
          hint: "Add case statement(s) to the empty switch, or remove",
        }
      ],
      r#"
switch (
  (() => {
    if (cond) {}
    return 42;
  })()
) {
  case 1:
    foo();
    break;
  default:
    bar();
    break;
}
      "#: [
        {
          line: 4,
          col: 14,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],

      // https://github.com/denoland/deno_lint/issues/469
      "try { foo(); } catch /* outside block */{ }": [
        {
          col: 40,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "try { foo(); } catch { }/* outside block */": [
        {
          col: 21,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      r#"
try {
  foo();
} catch {
}// pass
      "#: [
        {
          line: 4,
          col: 8,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ]
    };
  }
}
