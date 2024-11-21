// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{JSXElement, JSXElementName};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXVoidDomElementsNoChildren;

const CODE: &str = "jsx-void-dom-elements-no-children";

impl LintRule for JSXVoidDomElementsNoChildren {
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
    JSXVoidDomElementsNoChildrenHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_void_dom_elements_no_children.md")
  }
}

const MESSAGE: &str = "Found one or more unescaped entities in JSX text";
const HINT: &str = "Escape the '\">} characters respectively";

struct JSXVoidDomElementsNoChildrenHandler;

impl Handler for JSXVoidDomElementsNoChildrenHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    if let JSXElementName::Ident(name) = node.opening.name {
      if !node.children.is_empty()
        && matches!(
          name.sym().as_str(),
          "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
        )
      {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_void_dom_elements_no_children_valid() {
    assert_lint_ok! {
      JSXVoidDomElementsNoChildren,
      filename: "file:///foo.jsx",
      r#"<br />"#,
      r#"<img />"#,
    };
  }

  #[test]
  fn jsx_void_dom_elements_no_children_invalid() {
    assert_lint_err! {
      JSXVoidDomElementsNoChildren,
      filename: "file:///foo.jsx",
      r#"<area>foo</area>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<base>foo</base>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<br>foo</br>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<col>foo</col>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<embed>foo</embed>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<hr>foo</hr>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<img>foo</img>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<input>foo</input>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<link>foo</link>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<meta>foo</meta>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<param>foo</param>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<source>foo</source>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<track>foo</track>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<wbr>foo</wbr>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
