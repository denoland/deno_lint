// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElementName, JSXExpr,
  Lit,
};
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct JSXA11yHtmlHasLang;

const CODE: &str = "jsx-a11y-html-has-lang";

// TODO(deno_lint): The reference rule supports a `components` setting to map
// custom component names to `html`. Option support is deferred until
// deno_lint's JS-plugin option handling is ready, so only the built-in
// `html` element is checked.
impl LintRule for JSXA11yHtmlHasLang {
  fn tags(&self) -> Tags {
    &[tags::A11Y, tags::JSX, tags::REACT, tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXA11yHtmlHasLangHandler.traverse(program, context);
  }
}

const MISSING_PROP_MESSAGE: &str = "Missing `lang` attribute.";
const MISSING_PROP_HINT: &str = "Add a `lang` attribute to the `html` element whose value represents the primary language of document.";
const MISSING_VALUE_MESSAGE: &str = "Missing value for `lang` attribute.";
const MISSING_VALUE_HINT: &str =
  "Provide a meaningful value for the `lang` attribute.";

struct JSXA11yHtmlHasLangHandler;

impl Handler for JSXA11yHtmlHasLangHandler {
  fn jsx_opening_element(
    &mut self,
    node: &ast_view::JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let JSXElementName::Ident(name) = node.name else {
      return;
    };
    if name.sym() != "html" {
      return;
    }

    let mut lang_attr = None;
    for attr in node.attrs {
      let JSXAttrOrSpread::JSXAttr(attr) = attr else {
        continue;
      };
      let JSXAttrName::Ident(attr_name) = attr.name else {
        continue;
      };
      if attr_name.sym().eq_ignore_ascii_case("lang") {
        lang_attr = Some(attr);
        break;
      }
    }

    let Some(attr) = lang_attr else {
      ctx.add_diagnostic_with_hint(
        node.name.range(),
        CODE,
        MISSING_PROP_MESSAGE,
        MISSING_PROP_HINT,
      );
      return;
    };

    if !is_valid_lang_value(attr.value) {
      ctx.add_diagnostic_with_hint(
        node.range(),
        CODE,
        MISSING_VALUE_MESSAGE,
        MISSING_VALUE_HINT,
      );
    }
  }
}

fn is_valid_lang_value(value: Option<JSXAttrValue>) -> bool {
  match value {
    // `<html lang />` is considered valid by the reference implementation.
    None => true,
    Some(JSXAttrValue::Str(s)) => !s.value().to_string_lossy().is_empty(),
    Some(JSXAttrValue::JSXExprContainer(container)) => {
      let JSXExpr::Expr(expr) = container.expr else {
        return false;
      };
      match expr {
        Expr::Lit(Lit::Null(_))
        | Expr::Lit(Lit::Bool(_))
        | Expr::Lit(Lit::Num(_)) => false,
        Expr::Ident(id) => id.sym() != "undefined",
        Expr::Lit(Lit::Str(s)) => !s.value().to_string_lossy().is_empty(),
        Expr::Tpl(tpl) => {
          !tpl.exprs.is_empty()
            || tpl.quasis.iter().any(|q| !q.inner.raw.as_str().is_empty())
        }
        _ => true,
      }
    }
    _ => true,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn html_has_lang_valid() {
    assert_lint_ok! {
      JSXA11yHtmlHasLang,
      filename: "file:///foo.jsx",
      r#"<div />;"#,
      r#"<html lang="en" />"#,
      r#"<html lang="en-US" />"#,
      r#"<html lang={"en-US"} />"#,
      r#"<html lang={`en-US`} />"#,
      r#"<html lang={`${foo}`} />"#,
      r#"<html lang={foo} />;"#,
      r#"<html lang />;"#,
      r#"<HTML />;"#,
    };
  }

  #[test]
  fn html_has_lang_invalid() {
    assert_lint_err! {
      JSXA11yHtmlHasLang,
      filename: "file:///foo.jsx",
      r#"<html />;"#: [
        {
          col: 1,
          message: MISSING_PROP_MESSAGE,
          hint: MISSING_PROP_HINT,
        }
      ],
      r#"<html {...props} />;"#: [
        {
          col: 1,
          message: MISSING_PROP_MESSAGE,
          hint: MISSING_PROP_HINT,
        }
      ],
      r#"<html lang={undefined} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang={null} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang={false} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang={1} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang={''} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang={``} />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ],
      r#"<html lang="" />;"#: [
        {
          col: 0,
          message: MISSING_VALUE_MESSAGE,
          hint: MISSING_VALUE_HINT,
        }
      ]
    };
  }
}
