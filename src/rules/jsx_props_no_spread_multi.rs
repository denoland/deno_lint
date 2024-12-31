// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{JSXAttrOrSpread, JSXOpeningElement, NodeTrait};
use deno_ast::{SourceRange, SourceRanged};

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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXPropsNoSpreadMultiHandler.traverse(program, context);
  }
}

const MESSAGE: &str = "Duplicate spread attribute found";
const HINT: &str = "Remove this spread attribute";

struct JSXPropsNoSpreadMultiHandler;

impl Handler for JSXPropsNoSpreadMultiHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let mut seen: HashSet<&str> = HashSet::new();
    for attr in node.attrs {
      if let JSXAttrOrSpread::SpreadElement(spread) = attr {
        let text = spread.expr.text();
        if seen.contains(text) {
          ctx.add_diagnostic_with_fixes(
            spread.range(),
            CODE,
            MESSAGE,
            Some(HINT.to_string()),
            vec![LintFix {
              description: "Remove this spread attribute".into(),
              changes: vec![LintFixChange {
                new_text: "".into(),
                range: SourceRange {
                  start: attr.range().start - 2,
                  end: attr.range().end + 1,
                },
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
          col: 15,
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
          col: 15,
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
          col: 25,
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
