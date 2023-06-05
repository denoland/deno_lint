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
pub struct NoWindowGlobal;

const CODE: &str = "no-window-global";
const MESSAGE: &str =
  "window is deprecated and scheduled for removal in Deno 2.0";
const HINT: &str = "Instead, use `globalThis`";

impl LintRule for NoWindowGlobal {
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
  fn ident(&mut self, ident: &ast_view::Ident, ctx: &mut Context) {
    if_chain! {
      if ident.sym() == "window";
      if ctx.scope().is_global(&ident.to_id());
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
  fn no_window_global_valid() {
    assert_lint_ok! {
      NoWindowGlobal,
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
    };
  }

  #[test]
  fn no_window_global_invalid() {
    assert_lint_err! {
      NoWindowGlobal,
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
