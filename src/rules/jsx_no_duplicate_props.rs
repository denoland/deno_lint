// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  JSXAttributeItem, JSXAttributeName, JSXOpeningElement, Program,
};

#[derive(Debug)]
pub struct JSXNoDuplicateProps;

const CODE: &str = "jsx-no-duplicate-props";

impl LintRule for JSXNoDuplicateProps {
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
    let mut handler = JSXNoDuplicatedPropsHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Duplicate JSX attribute found.";
const HINT: &str = "Remove the duplicated attribute.";

struct JSXNoDuplicatedPropsHandler;

impl Handler<'_> for JSXNoDuplicatedPropsHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let mut seen: HashSet<&str> = HashSet::new();
    for attr in &node.attributes {
      if let JSXAttributeItem::Attribute(attr_node) = attr {
        if let JSXAttributeName::Identifier(id) = &attr_node.name {
          let name = id.name.as_str();
          if seen.contains(name) {
            ctx.add_diagnostic_with_hint(id.span, CODE, MESSAGE, HINT);
          }

          seen.insert(name);
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
  fn jsx_no_duplicate_props_valid() {
    assert_lint_ok! {
      JSXNoDuplicateProps,
      filename: "file:///foo.jsx",
      "<App a b />",
      "<div a b />",
    };
  }

  #[test]
  fn jsx_no_duplicate_props_invalid() {
    assert_lint_err! {
      JSXNoDuplicateProps,
      filename: "file:///foo.jsx",
      "<div a a />": [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "<App a a />": [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "<App a {...b} a />": [
        {
          col: 14,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "<div a {...b} a />": [
        {
          col: 14,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
