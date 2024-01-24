// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoWindow;

const CODE: &str = "no-window";
const MESSAGE: &str =
  "window is deprecated and scheduled for removal in Deno 2.0";
const HINT: &str = "Instead, use `globalThis`";

impl LintRule for NoWindow {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoWindowGlobalHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_window_global.md")
  }
}

struct NoWindowGlobalHandler;

impl Handler for NoWindowGlobalHandler {
  fn member_expr(&mut self, expr: &ast_view::MemberExpr, ctx: &mut Context) {
    if expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(ident) = &expr.obj;
      if ident.sym() == "window";
      if ctx.scope().is_global(&ident.inner.to_id());
      then {
        ctx.add_diagnostic_with_hint(
          ident.range(),
          CODE,
          MESSAGE,
          HINT,
        );
      }
    }
  }

  fn expr_stmt(&mut self, expr: &ast_view::ExprStmt, ctx: &mut Context) {
    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(ident) = &expr.expr;
      if ident.sym() == "window";
      if ctx.scope().is_global(&ident.inner.to_id());
      then {
        ctx.add_diagnostic_with_hint(
          ident.range(),
          CODE,
          MESSAGE,
          HINT,
        );
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
        }
      ],
      r#"window["fetch"]()"#: [
        {
          col: 0,
        }
      ],
      r#"window[`fetch`]()"#: [
        {
          col: 0,
        }
      ],
      r#"
function foo() {
  const window = 42;
  return window;
}
window;
      "#: [
        {
          col: 0,
          line: 6,
        }
      ],
    };
  }
}
