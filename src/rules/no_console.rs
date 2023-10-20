use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::Ident;
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoConsole;

const MESSAGE: &str = "`console` usage is not allowed.";
const CODE: &str = "no-console";

impl LintRule for NoConsole {
  fn tags(&self) -> &'static [&'static str] {
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
  fn ident(&mut self, id: &Ident, ctx: &mut Context) {
    if id.sym().as_ref() == "console" && ctx.scope().is_global(&id.to_id()) {
      ctx.add_diagnostic(id.range(), CODE, MESSAGE);
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
