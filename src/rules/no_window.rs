// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::tags;
use crate::tags::Tags;

use deno_ast::oxc::ast::ast::{
  ComputedMemberExpression, Expression, ExpressionStatement, Program,
  StaticMemberExpression,
};
use deno_ast::oxc::span::Span;
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoWindow;

const CODE: &str = "no-window";
const MESSAGE: &str = "Window is no longer available in Deno";
const HINT: &str = "Instead, use `globalThis`";
const FIX_DESC: &str = "Rename window to globalThis";

impl LintRule for NoWindow {
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
    let mut handler = NoWindowGlobalHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoWindowGlobalHandler;

impl NoWindowGlobalHandler {
  fn add_diagnostic(&self, ctx: &mut Context, range: Span) {
    ctx.add_diagnostic_with_fixes(
      range,
      CODE,
      MESSAGE,
      Some(HINT.to_string()),
      vec![LintFix {
        description: FIX_DESC.into(),
        changes: vec![LintFixChange {
          new_text: "globalThis".into(),
          range,
        }],
      }],
    );
  }

  fn is_window_global(
    &self,
    ident: &deno_ast::oxc::ast::ast::IdentifierReference,
    ctx: &Context,
  ) -> bool {
    if ident.name != "window" {
      return false;
    }
    // Check if the reference resolves to a local binding via OXC scoping.
    // If it resolves to a symbol, it's shadowed by a local declaration.
    if let Some(ref_id) = ident.reference_id.get() {
      let reference = ctx.scoping().get_reference(ref_id);
      if reference.symbol_id().is_some() {
        return false; // shadowed
      }
    }
    true
  }
}

impl Handler<'_> for NoWindowGlobalHandler {
  fn static_member_expression(
    &mut self,
    expr: &StaticMemberExpression,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let Expression::Identifier(ident) = &expr.object;
      if self.is_window_global(ident, ctx);
      then {
        self.add_diagnostic(ctx, ident.span);
      }
    }
  }

  fn computed_member_expression(
    &mut self,
    expr: &ComputedMemberExpression,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let Expression::Identifier(ident) = &expr.object;
      if self.is_window_global(ident, ctx);
      then {
        self.add_diagnostic(ctx, ident.span);
      }
    }
  }

  fn expression_statement(
    &mut self,
    expr: &ExpressionStatement,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let Expression::Identifier(ident) = &expr.expression;
      if self.is_window_global(ident, ctx);
      then {
        self.add_diagnostic(ctx, ident.span);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_window_valid() {
    assert_lint_ok! {
      NoWindow,
      "fetch();",
      "self.fetch();",
      "globalThis.fetch();",

      // `window` is shadowed
      "const window = 42; window.fetch();",
      r#"const window = 42; window["fetch"]();"#,
      r#"const window = 42; window[`fetch`]();"#,
      "const window = 42; window.alert();",
      r#"const window = 42; window["alert"]();"#,
      r#"const window = 42; window[`alert`]();"#,

      // https://github.com/denoland/deno_lint/issues/1232
      "const params: { window: number } = { window: 23 };",
      "x.window"
    };
  }

  #[test]
  fn no_window_invalid() {
    assert_lint_err! {
      NoWindow,
      MESSAGE,
      HINT,
      r#"window.fetch()"#: [
        {
          col: 0,
          fix: (FIX_DESC, "globalThis.fetch()"),
        }
      ],
      r#"window["fetch"]()"#: [
        {
          col: 0,
          fix: (FIX_DESC, r#"globalThis["fetch"]()"#),
        }
      ],
      r#"window[`fetch`]()"#: [
        {
          col: 0,
          fix: (FIX_DESC, "globalThis[`fetch`]()"),
        }
      ],
      r#"
function foo() {
  const window = 42;
  return window;
}
window;"#: [
        {
          col: 0,
          line: 6,
          fix: (FIX_DESC, "
function foo() {
  const window = 42;
  return window;
}
globalThis;"),
        }
      ],
      r#"window.console.log()"#: [
        {
          col: 0,
          fix: (FIX_DESC, "globalThis.console.log()"),
        }
      ],
    };
  }
}
