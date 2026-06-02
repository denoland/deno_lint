// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{JSXChild, JSXElement, JSXFragment, Program};

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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = JSXNoUselessFragmentHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Unnecessary Fragment detected";
const HINT: &str = "Remove this Fragment";

struct JSXNoUselessFragmentHandler;

impl Handler<'_> for JSXNoUselessFragmentHandler {
  // Check root fragments
  fn jsx_fragment(&mut self, node: &JSXFragment, ctx: &mut Context) {
    if node.children.is_empty() {
      ctx.add_diagnostic_with_hint(node.span, CODE, MESSAGE, HINT);
    } else if node.children.len() == 1 {
      if let Some(JSXChild::Element(_) | JSXChild::Fragment(_)) =
        &node.children.first()
      {
        ctx.add_diagnostic_with_hint(node.span, CODE, MESSAGE, HINT);
      }
    }
  }

  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    for child in &node.children {
      if let JSXChild::Fragment(frag) = child {
        ctx.add_diagnostic_with_hint(frag.span, CODE, MESSAGE, HINT);
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
