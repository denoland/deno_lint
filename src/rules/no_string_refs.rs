// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{
  JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXOpeningElement, Lit,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoStringRefs;

const CODE: &str = "no-string-refs";

impl LintRule for NoStringRefs {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx", "fresh"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoStringRefsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_string_refs.md")
  }
}

const MESSAGE: &str = "String refs are deprecated";
const HINT: &str = "Use a callback or 'useRef' instead.";

struct NoStringRefsHandler;

impl Handler for NoStringRefsHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    for attr in node.attrs {
      if let JSXAttrOrSpread::JSXAttr(attr) = attr {
        if let JSXAttrName::Ident(name) = attr.name {
          if name.sym() == "ref" {
            if let Some(value) = attr.value {
              if let JSXAttrValue::Lit(lit) = value {
                if let Lit::Str(_) = lit {
                  ctx.add_diagnostic_with_hint(
                    attr.range(),
                    CODE,
                    MESSAGE,
                    HINT,
                  );
                }
              }
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_string_refs_valid() {
    assert_lint_ok! {
      NoStringRefs,
      filename: "file:///foo.jsx",
      r#"<div ref={() => {}} />"#,
      r#"<App ref={() => {}} />"#,
      r#"<div ref={foo} />"#,
    };
  }

  #[test]
  fn no_string_refs_invalid() {
    assert_lint_err! {
      NoStringRefs,
      filename: "file:///foo.jsx",
      r#"<div ref="asdf" />"#: [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<App ref="asdf" />"#: [
        {
          col: 5,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
