// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{JSXAttributeItem, JSXOpeningElement, Program};
use deno_ast::oxc::span::{GetSpan, Span};

#[derive(Debug)]
pub struct JSXPropsNoSpreadMulti;

const CODE: &str = "jsx-props-no-spread-multi";

impl LintRule for JSXPropsNoSpreadMulti {
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
    let mut handler = JSXPropsNoSpreadMultiHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

const MESSAGE: &str = "Duplicate spread attribute found";
const HINT: &str = "Remove this spread attribute";

struct JSXPropsNoSpreadMultiHandler;

impl Handler<'_> for JSXPropsNoSpreadMultiHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let mut seen: HashSet<&str> = HashSet::new();
    for attr in &node.attributes {
      if let JSXAttributeItem::SpreadAttribute(spread) = attr {
        let arg_span = spread.argument.span();
        let text =
          &ctx.source_text()[arg_span.start as usize..arg_span.end as usize];
        if seen.contains(text) {
          ctx.add_diagnostic_with_fixes(
            spread.span,
            CODE,
            MESSAGE,
            Some(HINT.to_string()),
            vec![LintFix {
              description: "Remove this spread attribute".into(),
              changes: vec![LintFixChange {
                new_text: "".into(),
                range: Span::new(attr.span().start - 1, attr.span().end),
              }],
            }],
          );
        }

        seen.insert(text);
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
  fn jsx_props_no_spread_multi_valid() {
    assert_lint_ok! {
      JSXPropsNoSpreadMulti,
      filename: "file:///foo.jsx",
      r#"<div {...foo} />"#,
      r#"<div {...foo} {...bar} />"#,
      r#"<Foo {...foo} />"#,
      r#"<Foo {...foo} {...bar} />"#,
      r#"<Foo {...foo.bar} {...foo.bar.baz} />"#,
    };
  }

  #[test]
  fn jsx_props_no_spread_multi_invalid() {
    assert_lint_err! {
      JSXPropsNoSpreadMulti,
      filename: "file:///foo.jsx",
      r#"<div {...foo} {...foo} />"#: [
        {
          col: 14,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Remove this spread attribute",
            "<div {...foo} />"
          )
        }
      ],
      r#"<Foo {...foo} {...foo} />"#: [
        {
          col: 14,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Remove this spread attribute",
            "<Foo {...foo} />"
          )
        }
      ],
      r#"<div {...foo.bar.baz} a {...foo.bar.baz} />"#: [
        {
          col: 24,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Remove this spread attribute",
            "<div {...foo.bar.baz} a />"
          )
        }
      ],
    };
  }
}
