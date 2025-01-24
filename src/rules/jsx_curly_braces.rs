// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  Expr, JSXAttr, JSXAttrValue, JSXElement, JSXElementChild, JSXExpr, Lit,
  NodeTrait,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXCurlyBraces;

const CODE: &str = "jsx-curly-braces";

impl LintRule for JSXCurlyBraces {
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
    JSXCurlyBracesHandler.traverse(program, context);
  }
}

enum DiagnosticKind {
  CurlyAttribute,
  CurlyChild,
  MissingCurlyAttribute,
}

impl DiagnosticKind {
  fn message(&self) -> &'static str {
    match *self {
      DiagnosticKind::CurlyAttribute => "Curly braces are not needed here",
      DiagnosticKind::MissingCurlyAttribute => {
        "Missing curly braces around JSX attribute value"
      }
      DiagnosticKind::CurlyChild => {
        "Found curly braces around JSX child literal"
      }
    }
  }

  fn hint(&self) -> &'static str {
    match *self {
      DiagnosticKind::CurlyAttribute => {
        "Remove curly braces around JSX attribute"
      }
      DiagnosticKind::MissingCurlyAttribute => {
        "Remove curly braces around JSX child"
      }
      DiagnosticKind::CurlyChild => "Remove curly braces around JSX child",
    }
  }
}

struct JSXCurlyBracesHandler;

impl Handler for JSXCurlyBracesHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    let mut child_iter = node.children.iter().peekable();

    let mut skip_count = 0;
    while let Some(child) = child_iter.next() {
      if skip_count > 0 {
        skip_count -= 1;
        continue;
      }

      if let JSXElementChild::JSXExprContainer(child_expr) = child {
        if let JSXExpr::Expr(Expr::Lit(Lit::Str(lit_str))) = child_expr.expr {
          // Allowed if this node is at the end of a line
          // <div>{" "}
          // </div>
          if let Some(next) = child_iter.peek() {
            let line = ctx.text_info().line_index(child.end());
            let line_next_child = ctx.text_info().line_index(next.end());

            if line < line_next_child {
              skip_count += 1;
              continue;
            }
          }

          ctx.add_diagnostic_with_fixes(
            child.range(),
            CODE,
            DiagnosticKind::CurlyChild.message(),
            Some(DiagnosticKind::CurlyChild.hint().to_string()),
            vec![LintFix {
              description: "Remove curly braces around JSX child".into(),
              changes: vec![LintFixChange {
                new_text: lit_str.value().to_string().into(),
                range: child.range(),
              }],
            }],
          )
        }
      }
    }
  }

  fn jsx_attr(&mut self, node: &JSXAttr, ctx: &mut Context) {
    if let Some(value) = node.value {
      match value {
        JSXAttrValue::JSXExprContainer(expr) => {
          if let JSXExpr::Expr(Expr::Lit(Lit::Str(lit_str))) = expr.expr {
            ctx.add_diagnostic_with_fixes(
              value.range(),
              CODE,
              DiagnosticKind::CurlyAttribute.message(),
              Some(DiagnosticKind::CurlyAttribute.hint().to_string()),
              vec![LintFix {
                description: "Remove curly braces around JSX attribute value"
                  .into(),
                changes: vec![LintFixChange {
                  new_text: format!("\"{}\"", lit_str.value()).into(),
                  range: value.range(),
                }],
              }],
            );
          }
        }
        JSXAttrValue::JSXElement(jsx_el) => {
          ctx.add_diagnostic_with_fixes(
            value.range(),
            CODE,
            DiagnosticKind::MissingCurlyAttribute.message(),
            Some(DiagnosticKind::MissingCurlyAttribute.hint().to_string()),
            vec![LintFix {
              description: "Add curly braces around JSX attribute value".into(),
              changes: vec![LintFixChange {
                new_text: format!("{{{}}}", jsx_el.text()).into(),
                range: value.range(),
              }],
            }],
          );
        }
        _ => {}
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
  fn jsx_curly_braces_valid() {
    assert_lint_ok! {
      JSXCurlyBraces,
      filename: "file:///foo.jsx",
      "<div foo={2} />",
      r#"<div>foo{" "}
    </div>"#,
      r#"<div>foo{" "}
      bar</div>"#,
      r#"<div>
        foo{" "}
        <span />
      </div>"#,
    };
  }

  #[test]
  fn jsx_curly_braces_invalid() {
    assert_lint_err! {
      JSXCurlyBraces,
      filename: "file:///foo.jsx",
      "<div foo={'foo'} />": [
        {
          col: 9,
          message: DiagnosticKind::CurlyAttribute.message(),
          hint: DiagnosticKind::CurlyAttribute.hint(),
          fix: (
            "Remove curly braces around JSX attribute value",
            "<div foo=\"foo\" />"
          )
        }
      ],
      "<div foo=<div /> />": [
        {
          col: 9,
          message: DiagnosticKind::MissingCurlyAttribute.message(),
          hint: DiagnosticKind::MissingCurlyAttribute.hint(),
          fix: (
            "Add curly braces around JSX attribute value",
            "<div foo={<div />} />"
          )
        }
      ],
      r#"<div>{"foo"}</div>"#: [
        {
          col: 5,
          message: DiagnosticKind::CurlyChild.message(),
          hint: DiagnosticKind::CurlyChild.hint(),
          fix: (
            "Remove curly braces around JSX child",
            "<div>foo</div>"
          )
        }
      ],
    };
  }
}
