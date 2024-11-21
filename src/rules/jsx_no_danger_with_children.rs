// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{JSXAttrName, JSXAttrOrSpread, JSXElement};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXNoDangerWithChildren;

const CODE: &str = "jsx-no-danger-with-children";

impl LintRule for JSXNoDangerWithChildren {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx", "fresh"]
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

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_no_danger_with_children.md")
  }
}

const MESSAGE: &str =
  "Using JSX children together with 'dangerouslySetInnerHTML' is invalid";
const HINT: &str = "Remove the JSX children";

struct JSXNoDangerWithChildrenHandler;

impl Handler for JSXNoDangerWithChildrenHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for attr in node.opening.attrs {
      if let JSXAttrOrSpread::JSXAttr(attr) = attr {
        if let JSXAttrName::Ident(id) = attr.name {
          if id.sym() == "dangerouslySetInnerHTML" {
            if !node.children.is_empty() {
              ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
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
  fn jsx_no_danger_with_children_valid() {
    assert_lint_ok! {
      JSXNoDangerWithChildren,
      filename: "file:///foo.jsx",
      r#"<div dangerouslySetInnerHTML={{ __html: "foo" }} />"#,
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
