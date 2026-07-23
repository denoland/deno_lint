// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  BinaryOp, Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElementName,
  JSXExpr, Lit,
};
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct JSXA11yIframeHasTitle;

const CODE: &str = "jsx-a11y-iframe-has-title";

// TODO(deno_lint): The reference rule supports a `components` setting to map
// custom component names to `iframe`. Option support is deferred until
// deno_lint's JS-plugin option handling is ready, so only the built-in
// `iframe` element is checked.
impl LintRule for JSXA11yIframeHasTitle {
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
    JSXA11yIframeHasTitleHandler.traverse(program, context);
  }
}

const MESSAGE: &str = "Missing `title` attribute for the `iframe` element.";
const HINT: &str = "Provide a `title` property for the `iframe` element.";

struct JSXA11yIframeHasTitleHandler;

impl Handler for JSXA11yIframeHasTitleHandler {
  fn jsx_opening_element(
    &mut self,
    node: &ast_view::JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let JSXElementName::Ident(name) = node.name else {
      return;
    };
    if name.sym() != "iframe" {
      return;
    }

    let mut title_attr = None;
    for attr in node.attrs {
      let JSXAttrOrSpread::JSXAttr(attr) = attr else {
        continue;
      };
      let JSXAttrName::Ident(attr_name) = attr.name else {
        continue;
      };
      if attr_name.sym().eq_ignore_ascii_case("title") {
        title_attr = Some(attr);
        break;
      }
    }

    let Some(attr) = title_attr else {
      ctx.add_diagnostic_with_hint(node.name.range(), CODE, MESSAGE, HINT);
      return;
    };

    if !is_valid_title_value(attr.value) {
      ctx.add_diagnostic_with_hint(node.name.range(), CODE, MESSAGE, HINT);
    }
  }
}

fn is_valid_title_value(value: Option<JSXAttrValue>) -> bool {
  match value {
    Some(JSXAttrValue::Str(s)) => !s.value().to_string_lossy().is_empty(),
    Some(JSXAttrValue::JSXExprContainer(container)) => {
      let JSXExpr::Expr(expr) = container.expr else {
        return false;
      };
      match expr {
        Expr::Lit(Lit::Str(s)) => !s.value().to_string_lossy().is_empty(),
        Expr::Tpl(tpl) => {
          !tpl.exprs.is_empty()
            || tpl.quasis.iter().any(|q| !q.inner.raw.as_str().is_empty())
        }
        Expr::Ident(id) => id.sym() != "undefined",
        // These expressions are considered valid because their value cannot
        // be statically determined to be empty.
        Expr::Call(_)
        | Expr::Member(_)
        | Expr::Cond(_)
        | Expr::TaggedTpl(_)
        | Expr::New(_) => true,
        Expr::Bin(bin) => matches!(
          bin.op(),
          BinaryOp::LogicalAnd
            | BinaryOp::LogicalOr
            | BinaryOp::NullishCoalescing
        ),
        _ => false,
      }
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn iframe_has_title_valid() {
    assert_lint_ok! {
      JSXA11yIframeHasTitle,
      filename: "file:///foo.jsx",
      r#"<div />;"#,
      r#"<iframe title='Unique title' />"#,
      r#"<iframe title={foo} />"#,
      r#"<iframe title={`Title`} />"#,
      r#"<iframe title={`${title}`} />"#,
      r#"<FooComponent />"#,
      r#"<iframe title={titleGenerator('hello')} />"#,
      r#"<iframe title={file.name} />"#,
      r#"<iframe title={obj.prop.name} />"#,
      r#"<iframe title={obj['prop']} />"#,
      r#"<iframe title={a ?? b} />"#,
      r#"<iframe title={a && b} />"#,
      r#"<iframe title={a ? b : c} />"#,
      r#"<iframe title={i18n`title`} />"#,
      r#"<iframe title={new Title()} />"#,
    };
  }

  #[test]
  fn iframe_has_title_invalid() {
    assert_lint_err! {
      JSXA11yIframeHasTitle,
      filename: "file:///foo.jsx",
      r#"<iframe />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe {...props} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={undefined} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title='' />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={false} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={true} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={''} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={``} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<iframe title={42} />"#: [
        {
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
