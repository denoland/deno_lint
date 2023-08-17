use super::{ Context, LintRule };
use crate::handler::{ Handler, Traverse };
use crate::Program;
use deno_ast::view::{ CallExpr, Expr };

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
        if let Expr::Member(member_expr) = call_expr.callee() {
            if member_expr.object().as_ident().unwrap().raw() == "console" 
                && member_expr.prop().as_ident().unwrap().raw() == "log" 
            {
                ctx.add_diagnostic(call_expr.range(), CODE, MESSAGE);
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
        assert_lint_ok!(NoConsoleLog,
            r#"let foo = 0; const bar = 1;"#,
            r#"console.error('Error message');"#
        )
    }

    #[test]
}