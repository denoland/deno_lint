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
pub struct JSXA11yHeadingHasContent;

const CODE: &str = "jsx-a11y-heading-has-content";

// TODO(deno_lint): The reference rule supports a `components` option (and a
// `components` setting) to treat custom component names as headings. Option
// support is deferred until deno_lint's JS-plugin option handling is ready, so
// only the built-in `h1`-`h6` elements are checked.
impl LintRule for JSXA11yHeadingHasContent {
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
    JSXA11yHeadingHasContentHandler.traverse(program, context);
  }
}

const MESSAGE: &str =
  "Headings must have content and the content must be accessible by a screen reader.";
const HINT: &str =
  "Provide screen reader accessible content when using heading elements.";

struct JSXA11yHeadingHasContentHandler;

impl Handler for JSXA11yHeadingHasContentHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    let JSXElementName::Ident(name) = node.opening.name else {
      return;
    };

    if !is_heading(name.sym()) {
      return;
    }

    if object_has_accessible_child(node.children, node.opening) {
      return;
    }

    if is_hidden_from_screen_reader(node.opening) {
      return;
    }

    ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
  }
}

fn is_heading(name: &str) -> bool {
  matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
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
  fn heading_has_content_valid() {
    assert_lint_ok! {
      JSXA11yHeadingHasContent,
      filename: "file:///foo.jsx",
      r#"<h1>Foo</h1>"#,
      r#"<h2>Foo</h2>"#,
      r#"<h3>Foo</h3>"#,
      r#"<h4>Foo</h4>"#,
      r#"<h5>Foo</h5>"#,
      r#"<h6>Foo</h6>"#,
      r#"<h6>123</h6>"#,
      r#"<h1><Bar /></h1>"#,
      r#"<h1>{foo}</h1>"#,
      r#"<h1>{foo.bar}</h1>"#,
      r#"<h1 dangerouslySetInnerHTML={{ __html: "foo" }} />"#,
      r#"<h1 children={children} />"#,
      r#"<h1 aria-hidden />"#,
      r#"<h1><CustomInput type="hidden" /></h1>"#,
    };
  }

  #[test]
  fn heading_has_content_invalid() {
    assert_lint_err! {
      JSXA11yHeadingHasContent,
      filename: "file:///foo.jsx",
      r#"<h1 />"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<h1><Bar aria-hidden /></h1>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<h1>{undefined}</h1>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<h1><></></h1>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<h1><input type="hidden" /></h1>"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
