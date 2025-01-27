// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  JSXAttrName, JSXAttrOrSpread, JSXElement, JSXElementChild,
};
use deno_ast::SourceRanged;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct JSXNoDangerWithChildren;

const CODE: &str = "jsx-no-danger-with-children";

impl LintRule for JSXNoDangerWithChildren {
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
    JSXNoDangerWithChildrenHandler.traverse(program, context);
  }
}

const MESSAGE: &str =
  "Using JSX children together with 'dangerouslySetInnerHTML' is invalid";
const HINT: &str = "Remove the JSX children";

static IGNORE_TEXT: Lazy<regex::Regex> =
  Lazy::new(|| regex::Regex::new(r#"\n\s+"#).unwrap());

struct JSXNoDangerWithChildrenHandler;

impl Handler for JSXNoDangerWithChildrenHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for attr in node.opening.attrs {
      if let JSXAttrOrSpread::JSXAttr(attr) = attr {
        if let JSXAttrName::Ident(id) = attr.name {
          if id.sym() == "dangerouslySetInnerHTML" {
            let filtered = node
              .children
              .iter()
              .filter(|child| {
                if let JSXElementChild::JSXText(text) = child {
                  if IGNORE_TEXT.is_match(text.value()) {
                    return false;
                  }
                }

                true
              })
              .collect::<Vec<_>>();

            if !filtered.is_empty() {
              ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
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
      JSXNoDangerWithChildren,
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
      JSXNoDangerWithChildren,
      filename: "file:///foo.jsx",
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }}>foo</div>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT
        }
      ]
    };
  }
}
