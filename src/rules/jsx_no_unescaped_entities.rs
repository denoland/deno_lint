// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{JSXElement, JSXElementChild};
use deno_ast::SourceRanged;

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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXNoUnescapedEntitiesHandler.traverse(program, context);
  }
}

const MESSAGE: &str = "Found one or more unescaped entities in JSX text";
const HINT: &str = "Escape the '\">} characters respectively";

struct JSXNoUnescapedEntitiesHandler;

impl Handler for JSXNoUnescapedEntitiesHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for child in node.children {
      if let JSXElementChild::JSXText(jsx_text) = child {
        let text = jsx_text.raw().as_str();
        let new_text = text
          .replace('>', "&gt;")
          .replace('"', "&quot;")
          .replace('\'', "&apos;")
          .replace('}', "&#125;");

        if text != new_text {
          ctx.add_diagnostic_with_fixes(
            node.range(),
            CODE,
            MESSAGE,
            Some(HINT.to_string()),
            vec![LintFix {
              description: "Escape entities in the text node".into(),
              changes: vec![LintFixChange {
                new_text: new_text.into(),
                range: child.range(),
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
    };
  }

  #[test]
  fn jsx_no_unescaped_entities_invalid() {
    assert_lint_err! {
      JSXNoUnescapedEntities,
      filename: "file:///foo.jsx",
      r#"<div>'">}</div>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Escape entities in the text node",
            "<div>&apos;&quot;&gt;&#125;</div>"
          )
        }
      ]
    };
  }
}
