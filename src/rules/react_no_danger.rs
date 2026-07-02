// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{JSXAttribute, JSXAttributeName, Program};

#[derive(Debug)]
pub struct ReactNoDanger;

const CODE: &str = "react-no-danger";

impl LintRule for ReactNoDanger {
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
    let mut handler = NoDangerHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Do not use `dangerouslySetInnerHTML`";
const HINT: &str = "Remove this attribute";

struct NoDangerHandler;

impl Handler<'_> for NoDangerHandler {
  fn jsx_attribute(&mut self, node: &JSXAttribute, ctx: &mut Context) {
    if let JSXAttributeName::Identifier(name) = &node.name {
      if name.name == "dangerouslySetInnerHTML" {
        ctx.add_diagnostic_with_hint(name.span, CODE, MESSAGE, HINT);
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
      // A `{/* deno-lint-ignore */}` block comment suppresses the next
      // line, since `//` line comments aren't valid inside JSX children.
      // See https://github.com/denoland/deno_lint/issues/1452
      r#"<div>
        {/* deno-lint-ignore react-no-danger */}
        <div dangerouslySetInnerHTML={{}} />
      </div>"#,
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
