use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoConsole;

const MESSAGE: &str = "`console` usage is not allowed.";
const CODE: &str = "no-console";

impl LintRule for NoConsole {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoConsoleHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_console.md")
  }
}

struct NoConsoleHandler;

impl Handler for NoConsoleHandler {
  fn member_expr(&mut self, expr: &ast_view::MemberExpr, ctx: &mut Context) {
    if expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(ident) = &expr.obj;
      if ident.sym() == "console";
      if ctx.scope().is_global(&ident.inner.to_id());
      then {
        ctx.add_diagnostic(
          ident.range(),
          CODE,
          MESSAGE,
        );
      }
    }
  }

  fn expr_stmt(&mut self, expr: &ast_view::ExprStmt, ctx: &mut Context) {
    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(ident) = &expr.expr;
      if ident.sym() == "console";
      if ctx.scope().is_global(&ident.inner.to_id());
      then {
        ctx.add_diagnostic(
          ident.range(),
          CODE,
          MESSAGE,
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn console_allowed() {
    assert_lint_ok!(
      NoConsole,
      // ignored
      r"// deno-lint-ignore no-console\nconsole.error('Error message');",
      // not global
      r"const console = { log() {} } console.log('Error message');",
      // https://github.com/denoland/deno_lint/issues/1232
      "const x: { console: any } = { console: 21 }; x.console",
    );
  }

  #[test]
  fn no_console_invalid() {
    // Test cases where console is present
    assert_lint_err!(
        NoConsole,
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
        }],
        r#"function log() { console.debug('Log'); }"#: [{
            col: 17,
            message: MESSAGE,
        }],
        r#"console;"#: [{
            col: 0,
            message: MESSAGE,
        }],
        r#"console.warn("test");"#: [{
            col: 0,
            message: MESSAGE,
        }],
    );
  }
}
