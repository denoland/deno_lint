// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{JSXElement, JSXElementChild, JSXFragment};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXNoUselessFragment;

const CODE: &str = "jsx-no-useless-fragment";

impl LintRule for JSXNoUselessFragment {
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
    JSXNoUselessFragmentHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_no_useless_fragment.md")
  }
}

const MESSAGE: &str = "Unnecessary Fragment detected";
const HINT: &str = "Remove this Fragment";

struct JSXNoUselessFragmentHandler;

impl Handler for JSXNoUselessFragmentHandler {
  // Check root fragments
  fn jsx_fragment(&mut self, node: &JSXFragment, ctx: &mut Context) {
    if node.children.is_empty() {
      ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
    } else if node.children.len() == 1 {
      if let Some(
        JSXElementChild::JSXElement(_) | JSXElementChild::JSXFragment(_),
      ) = &node.children.first()
      {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
      }
    }
  }

  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for child in node.children {
      if let JSXElementChild::JSXFragment(frag) = child {
        ctx.add_diagnostic_with_hint(frag.range(), CODE, MESSAGE, HINT);
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
  fn jsx_no_useless_fragment_valid() {
    assert_lint_ok! {
      JSXNoUselessFragment,
      filename: "file:///foo.jsx",
      r#"<><div /><div /></>"#,
      r#"<>foo<div /></>"#,
      r#"<>{foo}</>"#,
      r#"<>{foo}bar</>"#,
    };
  }

  #[test]
  fn jsx_no_useless_fragment_invalid() {
    assert_lint_err! {
      JSXNoUselessFragment,
      filename: "file:///foo.jsx",
      r#"<></>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<><div /></>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<p>foo <>bar</></p>"#: [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<p>foo <><div /><div /></></p>"#: [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
