use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;

use deno_ast::oxc::ast::ast::{
  ComputedMemberExpression, Expression, ExpressionStatement,
  IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, Program,
  StaticMemberExpression,
};
use deno_ast::oxc::syntax::symbol::SymbolId;
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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoConsoleHandler {
      imported_console: HashSet::new(),
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoConsoleHandler {
  imported_console: HashSet<SymbolId>,
}

impl NoConsoleHandler {
  fn resolve_symbol_id(
    ident: &IdentifierReference,
    ctx: &Context,
  ) -> Option<SymbolId> {
    let ref_id = ident.reference_id.get()?;
    ctx.scoping().get_reference(ref_id).symbol_id()
  }

  fn is_console(&self, ident: &IdentifierReference, ctx: &Context) -> bool {
    if let Some(symbol_id) = Self::resolve_symbol_id(ident, ctx) {
      return self.imported_console.contains(&symbol_id);
    }

    ident.name.as_str() == "console"
  }
}

impl Handler<'_> for NoConsoleHandler {
  fn import_declaration(
    &mut self,
    import: &ImportDeclaration,
    _ctx: &mut Context,
  ) {
    if import.source.value.as_str() != "node:console" {
      return;
    }

    let Some(specifiers) = &import.specifiers else {
      return;
    };

    for specifier in specifiers {
      if let ImportDeclarationSpecifier::ImportDefaultSpecifier(default) =
        specifier
      {
        if let Some(symbol_id) = default.local.symbol_id.get() {
          self.imported_console.insert(symbol_id);
        }
      }
    }
  }

  fn static_member_expression(
    &mut self,
    expr: &StaticMemberExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &expr.object {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.span, CODE, MESSAGE);
      }
    }
  }

  fn computed_member_expression(
    &mut self,
    expr: &ComputedMemberExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &expr.object {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.span, CODE, MESSAGE);
      }
    }
  }

  fn expression_statement(
    &mut self,
    expr: &ExpressionStatement,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &expr.expression {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.span, CODE, MESSAGE);
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
      r"const console = { log() {} }; console.log('Error message');",
      // https://github.com/denoland/deno_lint/issues/1232
      "const x: { console: any } = { console: 21 }; x.console",
      r#"import { Console } from "node:console"; Console;"#,
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
        "import console from \"node:console\";\nconsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        "import myConsole from \"node:console\";\nmyConsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        "import console from \"node:console\";\nconsole;": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
    );
  }
}
