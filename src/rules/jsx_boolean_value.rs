// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::Handler;
use crate::tags::Tags;
use crate::tags;
use deno_ast::oxc::ast::ast::{
  JSXAttribute, JSXAttributeValue, JSXExpression, Program,
};
use deno_ast::oxc::span::{GetSpan, Span};

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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = JSXBooleanValueHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str =
  "Passing 'true' to boolean attributes is the same as not passing it`";
const HINT: &str = "Remove the attribute value";
const FIX_DESC: &str = HINT;

struct JSXBooleanValueHandler;

impl Handler<'_> for JSXBooleanValueHandler {
  fn jsx_attribute(&mut self, node: &JSXAttribute, ctx: &mut Context) {
    if let Some(value) = &node.value {
      if let JSXAttributeValue::ExpressionContainer(expr) = value {
        if let JSXExpression::BooleanLiteral(lit_bool) = &expr.expression {
          if lit_bool.value {
            // Check that there are no comments within the expression container
            let has_comments = ctx.all_comments().any(|c| {
              c.span.start > expr.span.start && c.span.end < expr.span.end
            });
            if has_comments {
              return;
            }

            // Build fix: remove from the attribute name end to the expression container end
            let attr_name_span = node.name.span();
            let value_span = value.span();
            let mut fixes = Vec::with_capacity(1);
            fixes.push(LintFix {
              description: FIX_DESC.into(),
              changes: vec![LintFixChange {
                new_text: "".into(),
                range: Span::new(attr_name_span.end, value_span.end),
              }],
            });
            ctx.add_diagnostic_with_fixes(
              value.span(),
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
