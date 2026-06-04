// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  JSXAttributeItem, JSXAttributeName, JSXChild, JSXElement, Program,
};
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct ReactNoDangerWithChildren;

const CODE: &str = "react-no-danger-with-children";

impl LintRule for ReactNoDangerWithChildren {
  fn tags(&self) -> Tags {
    &[tags::REACT, tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = JSXNoDangerWithChildrenHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str =
  "Using JSX children together with 'dangerouslySetInnerHTML' is invalid";
const HINT: &str = "Remove the JSX children";

static IGNORE_TEXT: Lazy<regex::Regex> =
  Lazy::new(|| regex::Regex::new(r#"^\n\s+$"#).unwrap());

struct JSXNoDangerWithChildrenHandler;

impl Handler<'_> for JSXNoDangerWithChildrenHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for attr in &node.opening_element.attributes {
      if let JSXAttributeItem::Attribute(attr) = attr {
        if let JSXAttributeName::Identifier(id) = &attr.name {
          if id.name == "dangerouslySetInnerHTML" {
            let filtered = node
              .children
              .iter()
              .filter(|child| {
                if let JSXChild::Text(text) = child {
                  if IGNORE_TEXT.is_match(text.value.as_str()) {
                    return false;
                  }
                }

                true
              })
              .collect::<Vec<_>>();

            if !filtered.is_empty() {
              ctx.add_diagnostic_with_hint(id.span, CODE, MESSAGE, HINT);
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_no_danger_with_children_valid() {
    assert_lint_ok! {
      ReactNoDangerWithChildren,
      filename: "file:///foo.jsx",
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }} />"#,
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }}></div>"#,
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }}>
      </div>"#,
    };
  }

  #[test]
  fn jsx_no_danger_with_children_invalid() {
    assert_lint_err! {
      ReactNoDangerWithChildren,
      filename: "file:///foo.jsx",
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }}>foo</div>"#: [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT
        }
      ]
    };
  }
}
