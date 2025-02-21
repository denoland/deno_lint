// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::{tags, Program};
use deno_ast::swc::parser::token::Token;
use deno_ast::view::{AssignOp, Expr, JSXAttr, JSXAttrValue, JSXExpr, Lit};
use deno_ast::{SourceRange, SourceRanged, SourceRangedForSpanned};

#[derive(Debug)]
pub struct JSXBooleanValue;

const CODE: &str = "jsx-boolean-value";

impl LintRule for JSXBooleanValue {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX]
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
}

const MESSAGE: &str =
  "Passing 'true' to boolean attributes is the same as not passing it`";
const HINT: &str = "Remove the attribute value";
const FIX_DESC: &str = HINT;

struct JSXBooleanValueHandler;

impl Handler for JSXBooleanValueHandler {
  fn jsx_attr(&mut self, node: &JSXAttr, ctx: &mut Context) {
    if let Some(value) = node.value {
      if let JSXAttrValue::JSXExprContainer(expr) = value {
        if let JSXExpr::Expr(Expr::Lit(Lit::Bool(lit_bool))) = expr.expr {
          if lit_bool.value()
            && lit_bool.leading_comments_fast(ctx.program()).is_empty()
            && lit_bool.trailing_comments_fast(ctx.program()).is_empty()
          {
            let mut fixes = Vec::with_capacity(1);
            if let Some(token) = expr.previous_token_fast(ctx.program()) {
              if token.token == Token::AssignOp(AssignOp::Assign) {
                let start_pos = token
                  .previous_token_fast(ctx.program())
                  .map(|t| t.end())
                  .unwrap_or(token.start());
                fixes.push(LintFix {
                  description: FIX_DESC.into(),
                  changes: vec![LintFixChange {
                    new_text: "".into(),
                    range: SourceRange::new(start_pos, expr.end()),
                  }],
                });
              }
            }
            ctx.add_diagnostic_with_fixes(
              value.range(),
              CODE,
              MESSAGE,
              Some(HINT.into()),
              fixes,
            );
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
      "<Foo foo={/* some comment */ true} />",
      "<Foo foo={true /* some comment */} />",
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
          fix: (FIX_DESC, "<Foo foo />"),
        }
      ],
    };

    assert_lint_err! {
      JSXBooleanValue,
      filename: "file:///foo.jsx",
      "<Foo foo = { true } />": [
        {
          col: 11,
          message: MESSAGE,
          hint: HINT,
          fix: (FIX_DESC, "<Foo foo />"),
        }
      ],
    };
  }
}
