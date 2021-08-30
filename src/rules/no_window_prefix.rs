// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use if_chain::if_chain;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use swc_atoms::JsWord;
use swc_common::Spanned;
use swc_ecmascript::utils::ident::IdentLike;

pub struct NoWindowPrefix;

const CODE: &str = "no-window-prefix";
const MESSAGE: &str = "For compatibility between the Window context and the Web Workers, calling Web APIs via `window` is disalloed";
const HINT: &str =
  "Instead, call this API via `self`, `globalThis`, or no extra prefix";

impl LintRule for NoWindowPrefix {
  fn new() -> Box<Self> {
    Box::new(NoWindowPrefix)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoWindowPrefixHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_window_prefix.md")
  }
}

static ALLOWED_PROPERTIES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  [
    "onload",
    "onunload",
    "closed",
    "alert",
    "confirm",
    "prompt",
    "localStorage",
    "sessionStorage",
    "window",
    "Navigator",
  ]
  .iter()
  .copied()
  .collect()
});

/// Extracts a symbol from the given expression if the symbol is statically determined (otherwise,
/// return `None`).
fn extract_symbol<'a>(expr: &'a ast_view::MemberExpr) -> Option<&'a JsWord> {
  use ast_view::{Expr, Lit, Tpl};
  match &expr.prop {
    Expr::Lit(Lit::Str(s)) => Some(s.value()),
    // If it's computed, this MemberExpr looks like `foo[bar]`
    Expr::Ident(ident) if !expr.computed() => Some(ident.sym()),
    Expr::Tpl(Tpl {
      ref exprs,
      ref quasis,
      ..
    }) if exprs.is_empty() && quasis.len() == 1 => Some(quasis[0].raw.value()),
    _ => None,
  }
}

struct NoWindowPrefixHandler;

impl Handler for NoWindowPrefixHandler {
  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    // Don't check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use ast_view::{Expr, ExprOrSuper};
    if_chain! {
      if let ExprOrSuper::Expr(Expr::Ident(obj)) = &member_expr.obj;
      let obj_symbol = obj.sym();
      if obj_symbol == "window";
      if ctx.scope().is_global(&obj.inner.to_id());
      if let Some(prop_symbol) = extract_symbol(member_expr);
      if !ALLOWED_PROPERTIES.contains(prop_symbol.as_ref());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.span(),
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
  fn no_deprecated_deno_api_valid() {
    assert_lint_ok! {
      NoWindowPrefix,
      "fetch();",
      "self.fetch();",
      "globalThis.fetch();",

      "Deno.metrics();",
      "self.Deno.metrics();",
      "globalThis.Deno.metrics();",

      "onload();",
      "self.onload();",
      "globalThis.onload();",
      "window.onload();",
      r#"window["onload"]();"#,
      r#"window[`onload`]();"#,

      "onunload();",
      "self.onunload();",
      "globalThis.onunload();",
      "window.onunload();",
      r#"window["onunload"]();"#,
      r#"window[`onunload`]();"#,

      "closed;",
      "self.closed;",
      "globalThis.closed;",
      "window.closed;",
      r#"window["closed"];"#,
      r#"window[`closed`];"#,

      "alert();",
      "self.alert();",
      "globalThis.alert();",
      "window.alert();",
      r#"window["alert"]();"#,
      r#"window[`alert`]();"#,

      "confirm();",
      "self.confirm();",
      "globalThis.confirm();",
      "window.confirm();",
      r#"window["confirm"]();"#,
      r#"window[`confirm`]();"#,

      "prompt();",
      "self.prompt();",
      "globalThis.prompt();",
      "window.prompt();",
      r#"window["prompt"]();"#,
      r#"window[`prompt`]();"#,

      "localStorage;",
      "self.localStorage;",
      "globalThis.localStorage;",
      "window.localStorage;",
      r#"window["localStorage"];"#,
      r#"window[`localStorage`];"#,

      "sessionStorage;",
      "self.sessionStorage;",
      "globalThis.sessionStorage;",
      "window.sessionStorage;",
      r#"window["sessionStorage"];"#,
      r#"window[`sessionStorage`];"#,

      "window;",
      "self.window;",
      "globalThis.window;",
      "window.window;",
      r#"window["window"];"#,
      r#"window[`window`];"#,

      "Navigator;",
      "self.Navigator;",
      "globalThis.Navigator;",
      "window.Navigator;",
      r#"window["Navigator"];"#,
      r#"window[`Navigator`];"#,

      // `window` is shadowed
      "const window = 42; window.fetch();",
      r#"const window = 42; window["fetch"]();"#,
      r#"const window = 42; window[`fetch`]();"#,
      "const window = 42; window.alert();",
      r#"const window = 42; window["alert"]();"#,
      r#"const window = 42; window[`alert`]();"#,

      // Ignore property access with variables
      r#"const f = "fetch"; window[f]();"#,
      r#"const f = "fetch"; window[`${f}`]();"#,

      // Make sure that no false positives are triggered on chained member
      // expressions
      r#"foo.window.fetch();"#,
    };
  }

  #[test]
  fn no_deprecated_deno_api_invalid() {
    assert_lint_err! {
      NoWindowPrefix,
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
window.fetch();
      "#: [
        {
          col: 0,
          line: 6,
        }
      ],
    };
  }
}
