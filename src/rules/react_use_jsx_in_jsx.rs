// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};

pub struct ReactUseJsxInJsx;

const CODE: &str = "react-use-jsx-in-jsx";
const MESSAGE: &str =
  "Do not call React components as functions from within JSX context";
const HINT: &str = "Use JSX";

impl LintRule for ReactUseJsxInJsx {
  fn new() -> Box<Self> {
    Box::new(ReactUseJsxInJsx)
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
    ReactUseJsxInJsxHandler::new().traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/react_use_jsx_in_jsx.md")
  }
}

struct ReactUseJsxInJsxHandler {
  is_in_jsx: u128,
}

impl ReactUseJsxInJsxHandler {
  fn new() -> Self {
    Self { is_in_jsx: 0 }
  }
}

use ast_view::{Expr, ExprOrSuper};

impl Handler for ReactUseJsxInJsxHandler {
  fn on_enter_node(&mut self, _n: ast_view::Node, _ctx: &mut Context) {
    if let ast_view::Node::JSXElement(_el) = _n {
      self.is_in_jsx += 1;
    }
  }
  fn on_exit_node(&mut self, _n: ast_view::Node, _ctx: &mut Context) {
    if let ast_view::Node::JSXElement(_el) = _n {
      self.is_in_jsx -= 1;
    }
  }
  fn call_expr(&mut self, n: &ast_view::CallExpr, ctx: &mut Context) {
    if self.is_in_jsx == 0 {
      return;
    }
    if let ExprOrSuper::Expr(Expr::Ident(id)) = n.callee {
      let sym = id.sym();
      let ch = sym
        .chars()
        .into_iter()
        .next()
        .expect("char missing in symbol");
      if ch.is_uppercase() || sym.starts_with("render") {
        ctx.add_diagnostic_with_hint(n.inner.span, CODE, MESSAGE, HINT);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn react_use_jsx_in_jsx_valid_jsx() {
    assert_lint_ok! {
      ReactUseJsxInJsx,
      filename: "foo.tsx",
      r#"<div><Hi /></div>"#,
    };
  }

  #[test]
  fn react_use_jsx_in_jsx_valid_non_component_fn_call() {
    assert_lint_ok! {
      ReactUseJsxInJsx,
      filename: "foo.tsx",
      r#"<div>{ok()}</div>"#,
    };
  }

  #[test]
  fn react_use_jsx_in_jsx_valid_non_component_field_fn_call() {
    assert_lint_ok! {
      ReactUseJsxInJsx,
      filename: "foo.tsx",
      r#"<div>{a.b.c()}</div>"#,
    };
  }

  #[test]
  fn react_use_jsx_in_jsx_valid_non_jsx_context() {
    assert_lint_ok! {
      ReactUseJsxInJsx,
      filename: "foo.tsx",
      r#"React.createElement("div", {}, MyComponent());"#,
    };
  }

  #[test]
  fn react_use_jsx_in_jsx_invalid() {
    assert_lint_err! {
      ReactUseJsxInJsx,
      filename: "foo.tsx",
      r#"<div>{Hi()}</div>"#: [
        {
          line: 1,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        },
      ],
    };
  }
}
