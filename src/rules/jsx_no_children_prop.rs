// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  JSXAttributeItem, JSXAttributeName, JSXOpeningElement, Program,
};

#[derive(Debug)]
pub struct JSXNoChildrenProp;

const CODE: &str = "jsx-no-children-prop";

impl LintRule for JSXNoChildrenProp {
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
    let mut handler = JSXNoChildrenPropHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Avoid passing children as a prop";

struct JSXNoChildrenPropHandler;

impl Handler<'_> for JSXNoChildrenPropHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    for attr in &node.attributes {
      if let JSXAttributeItem::Attribute(attr) = attr {
        if let JSXAttributeName::Identifier(id) = &attr.name {
          if id.name.as_str() == "children" {
            ctx.add_diagnostic(attr.span, CODE, MESSAGE);
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
