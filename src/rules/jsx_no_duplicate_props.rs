// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{JSXAttrName, JSXAttrOrSpread, JSXOpeningElement};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXNoDuplicateProps;

const CODE: &str = "jsx-no-duplicate-props";

impl LintRule for JSXNoDuplicateProps {
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
    JSXNoDuplicatedPropsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_no_duplicate_props.md")
  }
}

const MESSAGE: &str = "Duplicate JSX attribute found.";
const HINT: &str = "Remove the duplicated attribute.";

struct JSXNoDuplicatedPropsHandler;

impl Handler for JSXNoDuplicatedPropsHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let mut seen: HashSet<&'_ str> = HashSet::new();
    for attr in node.attrs {
      if let JSXAttrOrSpread::JSXAttr(attr_name) = attr {
        if let JSXAttrName::Ident(id) = attr_name.name {
          let name = id.sym().as_str();
          if seen.contains(name) {
            ctx.add_diagnostic_with_hint(id.range(), CODE, MESSAGE, HINT);
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
