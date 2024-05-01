use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CallExpr, NodeTrait};
use deno_ast::{SourceRanged};

#[derive(Debug)]
pub struct NoBooleanLiteralForArguments;

const CODE: &str = "no-boolean-literal-for-arguments";
const MESSAGE: &str = "Please create a self-documenting constant instead of \
passing plain boolean values as parameters";
const HINT: &str = "const ARG_ONE = true, ARG_TWO = false; yourFunction(ARG_ONE, ARG_TWO)";

impl LintRule for NoBooleanLiteralForArguments {
  fn lint_program_with_ast_view<'view>(&self, context: &mut Context<'view>, program: Program<'view>) {
    NoBooleanLiteralForArgumentsVisitor.traverse(program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_boolean_literal_for_arguments.md")
  }
}

struct NoBooleanLiteralForArgumentsVisitor;

impl Handler for NoBooleanLiteralForArgumentsVisitor {
  fn call_expr(&mut self, call_expression: &CallExpr, ctx: &mut Context) {
    let args = call_expression.args;
    let amount_of_args = args.len();
    if amount_of_args < 2 {
      return;
    }
    let mut total_amount_of_bool_args = 0;
    let is_boolean = |text: &str| -> bool {
      match text {
        "true" | "false" => true,
        _ => false
      }
    };
    for arg in args {
      if total_amount_of_bool_args > 0 {
        ctx.add_diagnostic_with_hint(
         call_expression.range(),
          CODE,
          MESSAGE,
          HINT,
        );
        break;
      }
      if is_boolean(arg.text()) {
        total_amount_of_bool_args += 1;
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
      r#"enable(true)"#,
      r#"enable(false)"#,
      r#"runCMDCommand(command, executionMode)"#,
      r#"
      function formatLog(logData: { level: string, text: string }) {
        console.log(`[${level}]:${text}`);
      }
      formatLog({ level: "INFO", text: "Connected to the DB!" });
      "#,
      r#"
      function displayInformation(display: boolean) {
        if (display) {
          renderInformation();
        }
        // TODO!
      }
      displayInformation(true);
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
    };
  }
}
