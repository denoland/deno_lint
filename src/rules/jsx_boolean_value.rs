// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{Expr, JSXAttr, JSXAttrValue, JSXExpr, Lit};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXBooleanValue;

const CODE: &str = "jsx-boolean-value";

impl LintRule for JSXBooleanValue {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXBooleanValueHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_boolean_value.md")
  }
}

const MESSAGE: &str =
  "Passing 'true' to boolean attributes is the same as not passing it`";
const HINT: &str = "Remove the attribute value";

struct JSXBooleanValueHandler;

impl Handler for JSXBooleanValueHandler {
  fn jsx_attr(&mut self, node: &JSXAttr, ctx: &mut Context) {
    if let Some(value) = node.value {
      if let JSXAttrValue::JSXExprContainer(expr) = value {
        if let JSXExpr::Expr(expr) = expr.expr {
          if let Expr::Lit(lit) = expr {
            if let Lit::Bool(lit_bool) = lit {
              if lit_bool.value() {
                ctx.add_diagnostic_with_hint(
                  value.range(),
                  CODE,
                  MESSAGE,
                  HINT,
                );
              }
            }
          }
        }
      }
    }
  }
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_no_comment_text_nodes_valid() {
    assert_lint_ok! {
      JSXBooleanValue,
      filename: "file:///foo.jsx",
      // non derived classes.
      "<Foo foo={false} />",
      "<Foo foo />",
    };
  }

  #[test]
  fn jsx_no_comment_text_nodes_invalid() {
    assert_lint_err! {
      JSXBooleanValue,
      filename: "file:///foo.jsx",
      "<Foo foo={true} />": [
        {
          col: 9,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
