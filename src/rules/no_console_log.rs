use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CallExpr, Expr, MemberProp};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoConsoleLog;

const MESSAGE: &str = "'console.log` calls are not allowed.";
const CODE: &str = "no-console";

impl LintRule for NoConsoleLog {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoConsoleLogHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_console_log.md")
  }
}

struct NoConsoleLogHandler;

impl Handler for NoConsoleLogHandler {
  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let deno_ast::view::Callee::Expr(Expr::Member(member_expr)) =
      &call_expr.callee
    {
      if let Expr::Ident(obj_ident) = &member_expr.obj {
        if obj_ident.sym().as_ref() == "console" {
          if let MemberProp::Ident(prop_ident) = &member_expr.prop {
            if prop_ident.sym().as_ref() == "log" {
              ctx.add_diagnostic(call_expr.range(), CODE, MESSAGE);
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_console_log_valid() {
    // Test cases where a console.log call is not present
    assert_lint_ok!(
      NoConsoleLog,
      r#"let foo = 0; const bar = 1;"#,
      r#"console.error('Error message');"#
    );
  }

  #[test]
  fn no_console_log_invalid() {
    // Test cases where console.log is present
    assert_lint_err!(
        NoConsoleLog,
        r#"console.log('Debug message');"#: [{
            col: 0,
            message: MESSAGE,
        }],
        r#"if (debug) { console.log('Debugging'); }"#: [{
            col: 13,
            message: MESSAGE,
        }],
        r#"function log() { console.log('Log'); }"#: [{
            col: 17,
            message: MESSAGE,
        }]
    );
  }
}
