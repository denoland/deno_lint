// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{JSXChild, JSXElement, Program};
use deno_ast::oxc::span::GetSpan;

#[derive(Debug)]
pub struct JSXNoUnescapedEntities;

const CODE: &str = "jsx-no-unescaped-entities";

impl LintRule for JSXNoUnescapedEntities {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX, tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = JSXNoUnescapedEntitiesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Found one or more unescaped entities in JSX text";
const HINT: &str = "Escape the >} characters respectively";

struct JSXNoUnescapedEntitiesHandler;

impl Handler<'_> for JSXNoUnescapedEntitiesHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for child in &node.children {
      if let JSXChild::Text(jsx_text) = child {
        let text = jsx_text.value.as_str();
        let new_text = text.replace('>', "&gt;").replace('}', "&#125;");

        if text != new_text {
          ctx.add_diagnostic_with_fixes(
            jsx_text.span,
            CODE,
            MESSAGE,
            Some(HINT.to_string()),
            vec![LintFix {
              description: "Escape entities in the text node".into(),
              changes: vec![LintFixChange {
                new_text: new_text.into(),
                range: child.span(),
              }],
            }],
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_no_unescaped_entities_valid() {
    assert_lint_ok! {
      JSXNoUnescapedEntities,
      filename: "file:///foo.jsx",
      r#"<div>&gt;</div>"#,
      r#"<div>{">"}</div>"#,
      r#"<div>{"}"}</div>"#,
    };
  }

}
