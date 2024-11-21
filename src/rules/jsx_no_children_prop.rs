// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{JSXAttrName, JSXAttrOrSpread, JSXOpeningElement};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXNoChildrenProp;

const CODE: &str = "jsx-no-children-prop";

impl LintRule for JSXNoChildrenProp {
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
    JSXNoChildrenPropHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_no_children_prop.md")
  }
}

const MESSAGE: &str = "Avoid passing children as a prop";

struct JSXNoChildrenPropHandler;

impl Handler for JSXNoChildrenPropHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    for attr in node.attrs {
      if let JSXAttrOrSpread::JSXAttr(attr) = attr {
        if let JSXAttrName::Ident(id) = attr.name {
          if id.sym() == "children" {
            ctx.add_diagnostic(attr.range(), CODE, MESSAGE);
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
  fn jsx_no_children_prop_valid() {
    assert_lint_ok! {
      JSXNoChildrenProp,
      filename: "file:///foo.jsx",
      r#"<div>foo</div>"#,
      r#"<div><Foo /><Bar /></div>"#,
    };
  }

  #[test]
  fn jsx_no_children_prop_invalid() {
    assert_lint_err! {
      JSXNoChildrenProp,
      filename: "file:///foo.jsx",
      r#"<div children="foo" />"#: [
        {
          col: 5,
          message: MESSAGE,
        }
      ],
      r#"<Foo children="foo" />"#: [
        {
          col: 5,
          message: MESSAGE,
        }
      ],
      r#"<div children={[1, 2]} />"#: [
        {
          col: 5,
          message: MESSAGE,
        }
      ],
    };
  }
}
