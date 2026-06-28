use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;

use deno_ast::swc::ast::Id;
use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use std::collections::HashSet;

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
    NoConsoleHandler {
      imported_console: HashSet::new(),
    }
    .traverse(program, context);
  }
}

struct NoConsoleHandler {
  /// Bindings imported as the default export of `node:console`, e.g.
  /// `import console from "node:console";`. These refer to the same `console`
  /// object as the global, so usages should be flagged too.
  imported_console: HashSet<Id>,
}

impl NoConsoleHandler {
  fn is_console(&self, ident: &ast_view::Ident, ctx: &mut Context) -> bool {
    let id = ident.inner.to_id();
    // `console` imported from `node:console` (any local name).
    if self.imported_console.contains(&id) {
      return true;
    }
    // The global `console`.
    ident.sym() == "console" && ctx.scope().is_global(&id)
  }
}

impl Handler for NoConsoleHandler {
  fn import_decl(&mut self, import: &ast_view::ImportDecl, _ctx: &mut Context) {
    if import.src.value().to_string_lossy() != "node:console" {
      return;
    }
    for specifier in import.specifiers {
      if let ast_view::ImportSpecifier::Default(default) = specifier {
        self.imported_console.insert(default.local.to_id());
      }
    }
  }

  fn member_expr(&mut self, expr: &ast_view::MemberExpr, ctx: &mut Context) {
    if expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if let Expr::Ident(ident) = &expr.obj {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.range(), CODE, MESSAGE);
      }
    }
  }

  fn expr_stmt(&mut self, expr: &ast_view::ExprStmt, ctx: &mut Context) {
    use deno_ast::view::Expr;
    if let Expr::Ident(ident) = &expr.expr {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.range(), CODE, MESSAGE);
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
      // a `console` imported from somewhere other than `node:console`
      "import console from './my-console.ts';\nconsole.log('hi');",
      // a non-default import from `node:console` (e.g. the `Console` class)
      "import { Console } from 'node:console';\nconst c = new Console(process.stdout);",
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
        // https://github.com/denoland/deno_lint/issues/1316
        // `console` imported from `node:console`.
        "import console from \"node:console\";\nconsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        // imported under a different local name.
        "import myConsole from \"node:console\";\nmyConsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        // bare reference to the imported binding.
        "import console from \"node:console\";\nconsole;": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
    );
  }
}
