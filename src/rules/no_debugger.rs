// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{DebuggerStatement, Program};
use derive_more::Display;

#[derive(Debug)]
pub struct NoDebugger;

const CODE: &str = "no-debugger";

#[derive(Display)]
enum NoDebuggerMessage {
  #[display(fmt = "`debugger` statement is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoDebuggerHint {
  #[display(fmt = "Remove the `debugger` statement")]
  Remove,
}

impl LintRule for NoDebugger {
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
    let mut handler = NoDebuggerHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoDebuggerHandler;

impl Handler<'_> for NoDebuggerHandler {
  fn debugger_statement(
    &mut self,
    debugger_stmt: &DebuggerStatement,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_hint(
      debugger_stmt.span,
      CODE,
      NoDebuggerMessage::Unexpected,
      NoDebuggerHint::Remove,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_debugger_invalid() {
    assert_lint_err! {
      NoDebugger,
      r#"function asdf(): number { console.log("asdf"); debugger; return 1; }"#: [
        {
          col: 47,
          message: NoDebuggerMessage::Unexpected,
          hint: NoDebuggerHint::Remove,
        }
      ]
    };
  }
}
