use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{Argument, CallExpression, Program};

#[derive(Debug)]
pub struct NoBooleanLiteralForArguments;

const CODE: &str = "no-boolean-literal-for-arguments";
const MESSAGE: &str = "Please create a self-documenting constant instead of \
passing plain booleans values as arguments";
const HINT: &str =
  "const ARG_ONE = true, ARG_TWO = false;\nyourFunction(ARG_ONE, ARG_TWO)";

impl LintRule for NoBooleanLiteralForArguments {
  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoBooleanLiteralForArgumentsVisitor;
    crate::handler::traverse_program(&mut handler, program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[]
  }
}

struct NoBooleanLiteralForArgumentsVisitor;

impl Handler<'_> for NoBooleanLiteralForArgumentsVisitor {
  fn call_expression(
    &mut self,
    call_expression: &CallExpression,
    ctx: &mut Context,
  ) {
    for arg in &call_expression.arguments {
      if matches!(arg, Argument::BooleanLiteral(_)) {
        ctx.add_diagnostic_with_hint(
          call_expression.span,
          CODE,
          MESSAGE,
          HINT,
        );
        break;
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn no_boolean_literal_for_arguments_valid() {
    assert_lint_ok! {
      NoBooleanLiteralForArguments,
      r#"runCMDCommand(command, executionMode)"#,
      r#"
      function formatLog(logData: { level: string, text: string }) {
        console.log(`[${level}]:${text}`);
      }
      formatLog({ level: "INFO", text: "Connected to the DB!" });
      "#,
      r#"
      function displayInformation(display: { renderer: "terminal" | "screen", recursive: boolean }) {
        if (display) {
          renderInformation();
        }
        // TODO!
      }
      displayInformation({ renderer: "terminal", recursive: true });
      "#
    }
  }

  #[test]
  fn no_boolean_literal_for_arguments_invalid() {
    assert_lint_err! {
      NoBooleanLiteralForArguments,
      r#"test(true,true)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"test(false,true)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"test(false,false)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"invoke(true,remoteServerUrl,true)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"
      function enableLinting(enable: boolean, limitDepth: boolean) {
        if (enable) {
          linter.run();
        }
      }
      enableLinting(true,false);
      "#:[{line: 7, col: 6, message: MESSAGE, hint: HINT}],
      r#"
      runCMD(true, CMD.MODE_ONE)
      "#:[{line: 2, col: 6, message: MESSAGE, hint: HINT}],
      r#"
      function displayInformation(display: boolean) {
        if (display) {
          renderInformation();
        }
        // TODO!
      }
      displayInformation(true);
      "#:[{line: 8, col: 6, message: MESSAGE, hint: HINT}],
    }
  }
}
