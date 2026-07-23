// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElement,
  JSXElementChild, JSXElementName, JSXExpr, JSXOpeningElement, Lit,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXA11yAnchorHasContent;

const CODE: &str = "jsx-a11y-anchor-has-content";

// TODO(deno_lint): The reference rule supports a `components` setting to map
// custom component names to `a`. Option support is deferred until deno_lint's
// JS-plugin option handling is ready, so only the built-in `a` element is
// checked. The reference rule's autofix (removing `aria-hidden`/`hidden`
// attributes) is also not ported.
impl LintRule for JSXA11yAnchorHasContent {
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
    JSXA11yAnchorHasContentHandler.traverse(program, context);
  }
}

const MESSAGE: &str = "Missing accessible content when using `a` elements.";
const HINT: &str =
  "Provide screen reader accessible content when using `a` elements.";

struct JSXA11yAnchorHasContentHandler;

impl Handler for JSXA11yAnchorHasContentHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    let JSXElementName::Ident(name) = node.opening.name else {
      return;
    };

    if name.sym() != "a" {
      return;
    }

    if is_hidden_from_screen_reader(node.opening) {
      return;
    }

    if object_has_accessible_child(node.children, node.opening) {
      return;
    }

    for attr in ["title", "aria-label"] {
      if get_attr_ignore_case(node.opening, attr).is_some() {
        return;
      }
    }

    ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
  }
}

fn get_attr_ignore_case<'a>(
  opening: &'a JSXOpeningElement<'a>,
  target: &str,
) -> Option<&'a JSXAttr<'a>> {
  opening.attrs.iter().find_map(|attr| {
    let JSXAttrOrSpread::JSXAttr(attr) = attr else {
      return None;
    };
    let JSXAttrName::Ident(name) = attr.name else {
      return None;
    };
    if name.sym().eq_ignore_ascii_case(target) {
      Some(*attr)
    } else {
      None
    }
  })
}

fn is_hidden_from_screen_reader<'a>(
  opening: &'a JSXOpeningElement<'a>,
) -> bool {
  let JSXElementName::Ident(name) = opening.name else {
    return false;
  };

  if name.sym().eq_ignore_ascii_case("input") {
    if let Some(attr) = get_attr_ignore_case(opening, "type") {
      if let Some(JSXAttrValue::Str(s)) = attr.value {
        if s.value().to_string_lossy().eq_ignore_ascii_case("hidden") {
          return true;
        }
      }
    }
  }

  match get_attr_ignore_case(opening, "aria-hidden") {
    None => false,
    Some(attr) => match attr.value {
      None => true,
      Some(JSXAttrValue::Str(s)) => s.value().to_string_lossy() == "true",
      Some(JSXAttrValue::JSXExprContainer(container)) => {
        expr_to_boolean(&container.expr)
      }
      _ => false,
    },
  }
}

fn expr_to_boolean(expr: &JSXExpr) -> bool {
  let JSXExpr::Expr(expr) = expr else {
    return false;
  };
  match expr {
    Expr::Lit(Lit::Bool(b)) => b.value(),
    Expr::Lit(Lit::Num(n)) => n.value() != 0.0,
    Expr::Lit(Lit::Str(s)) => !s.value().to_string_lossy().is_empty(),
    _ => false,
  }
}

fn object_has_accessible_child<'a>(
  children: &'a [JSXElementChild<'a>],
  opening: &'a JSXOpeningElement<'a>,
) -> bool {
  let has_child = children.iter().any(|child| match child {
    JSXElementChild::JSXText(text) => !text.value().is_empty(),
    JSXElementChild::JSXElement(child) => {
      !is_hidden_from_screen_reader(child.opening)
    }
    JSXElementChild::JSXExprContainer(container) => match container.expr {
      JSXExpr::Expr(Expr::Lit(Lit::Null(_))) => false,
      JSXExpr::Expr(Expr::Ident(id)) => id.sym() != "undefined",
      JSXExpr::JSXEmptyExpr(_) => false,
      _ => true,
    },
    _ => false,
  });

  has_child
    || get_attr_ignore_case(opening, "dangerouslySetInnerHTML").is_some()
    || get_attr_ignore_case(opening, "children").is_some()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anchor_has_content_valid() {
    assert_lint_ok! {
      JSXA11yAnchorHasContent,
      filename: "file:///foo.jsx",
      r#"<div />;"#,
      r#"<a>Foo</a>"#,
      r#"<a><Bar /></a>"#,
      r#"<a>{foo}</a>"#,
      r#"<a>{foo.bar}</a>"#,
      r#"<a dangerouslySetInnerHTML={{ __html: "foo" }} />"#,
      r#"<a children={children} />"#,
      r#"<Link />"#,
      r#"<a title={title} />"#,
      r#"<a aria-label={ariaLabel} />"#,
      r#"<a title={title} aria-label={ariaLabel} />"#,
      r#"<a><Bar aria-hidden="false" /></a>"#,
      r#"<a aria-hidden>Foo</a>"#,
      r#"<a aria-hidden="true">Foo</a>"#,
      r#"<a hidden>Foo</a>"#,
      r#"<a aria-hidden><span aria-hidden>Foo</span></a>"#,
      r#"<a hidden="true">Foo</a>"#,
      r#"<a hidden="">Foo</a>"#,
      r#"<a><div hidden /></a>"#,
      r#"<a><Bar hidden /></a>"#,
      r#"<a><Bar hidden="" /></a>"#,
      r#"<a><Bar hidden="until-hidden" /></a>"#,
    };
  }

  #[test]
  fn anchor_has_content_invalid() {
    assert_lint_err! {
      JSXA11yAnchorHasContent,
      filename: "file:///foo.jsx",
      r#"<a />"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a><Bar aria-hidden /></a>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a><Bar aria-hidden="true" /></a>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a><input type="hidden" /></a>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a>{undefined}</a>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a>{null}</a>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
