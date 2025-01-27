// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{JSXAttr, JSXAttrName};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct ReactNoDanger;

const CODE: &str = "react-no-danger";

impl LintRule for ReactNoDanger {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoDangerHandler.traverse(program, context);
  }
}

const MESSAGE: &str = "Do not use `dangerouslySetInnerHTML`";
const HINT: &str = "Remove this attribute";

struct NoDangerHandler;

impl Handler for NoDangerHandler {
  fn jsx_attr(&mut self, node: &JSXAttr, ctx: &mut Context) {
    if let JSXAttrName::Ident(name) = node.name {
      if name.sym() == "dangerouslySetInnerHTML" {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
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
  fn no_danger_valid() {
    assert_lint_ok! {
      ReactNoDanger,
      filename: "file:///foo.jsx",
      // non derived classes.
      r#"<div />"#,
    };
  }

  #[test]
  fn no_danger_invalid() {
    assert_lint_err! {
      ReactNoDanger,
      filename: "file:///foo.jsx",
      "<div dangerouslySetInnerHTML />": [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<div dangerouslySetInnerHTML="" />"#: [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "<div dangerouslySetInnerHTML={{}} />": [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
