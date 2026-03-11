// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  JSXAttribute, JSXAttributeValue, JSXChild, JSXElement,
  JSXExpression, Program,
};
use deno_ast::oxc::span::GetSpan;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct JSXCurlyBraces;

const CODE: &str = "jsx-curly-braces";

static IGNORE_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[{}<>]").unwrap());

impl LintRule for JSXCurlyBraces {
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
    let mut handler = JSXCurlyBracesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
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

impl Handler<'_> for JSXCurlyBracesHandler {
  fn jsx_element(&mut self, node: &JSXElement, ctx: &mut Context) {
    let children = &node.children;
    let mut child_iter = children.iter().peekable();

    let mut skip_count = 0;
    while let Some(child) = child_iter.next() {
      if skip_count > 0 {
        skip_count -= 1;
        continue;
      }

      if let JSXChild::ExpressionContainer(child_expr) = child {
        if let JSXExpression::StringLiteral(lit_str) = &child_expr.expression {
          // Ignore entities which would require escaping.
          if IGNORE_CHARS.is_match(lit_str.value.as_str()) {
            continue;
          }

          // Allowed if this node is at the end of a line
          // <div>{" "}
          // </div>
          if let Some(next) = child_iter.peek() {
            let child_span = child_expr.span;
            let next_span = next.span();
            let line = ctx.text_info().line_index(child_span.end as usize);
            let line_next_child =
              ctx.text_info().line_index(next_span.end as usize);

            if line < line_next_child {
              skip_count += 1;
              continue;
            }
          }

          ctx.add_diagnostic_with_fixes(
            child_expr.span,
            CODE,
            DiagnosticKind::CurlyChild.message(),
            Some(DiagnosticKind::CurlyChild.hint().to_string()),
            vec![LintFix {
              description: "Remove curly braces around JSX child".into(),
              changes: vec![LintFixChange {
                new_text: lit_str.value.to_string().into(),
                range: child_expr.span,
              }],
            }],
          )
        }
      }
    }
  }

  fn jsx_attribute(&mut self, node: &JSXAttribute, ctx: &mut Context) {
    if let Some(value) = &node.value {
      match value {
        JSXAttributeValue::ExpressionContainer(expr) => {
          if let JSXExpression::StringLiteral(lit_str) = &expr.expression {
            ctx.add_diagnostic_with_fixes(
              value.span(),
              CODE,
              DiagnosticKind::CurlyAttribute.message(),
              Some(DiagnosticKind::CurlyAttribute.hint().to_string()),
              vec![LintFix {
                description: "Remove curly braces around JSX attribute value"
                  .into(),
                changes: vec![LintFixChange {
                  new_text: format!("\"{}\"", lit_str.value).into(),
                  range: value.span(),
                }],
              }],
            );
          }
        }
        JSXAttributeValue::Element(jsx_el) => {
          let el_text = &ctx.source_text()
            [jsx_el.span.start as usize..jsx_el.span.end as usize];
          ctx.add_diagnostic_with_fixes(
            value.span(),
            CODE,
            DiagnosticKind::MissingCurlyAttribute.message(),
            Some(DiagnosticKind::MissingCurlyAttribute.hint().to_string()),
            vec![LintFix {
              description: "Add curly braces around JSX attribute value".into(),
              changes: vec![LintFixChange {
                new_text: format!("{{{}}}", el_text).into(),
                range: value.span(),
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
      r#"<div>foo{"<"}</div>"#,
      r#"<div>foo{">"}</div>"#,
      r#"<div>foo{"}"}</div>"#,
      r#"<div>foo{"{"}</div>"#,
      r#"<div>foo{"foo <"}</div>"#,
      r#"<div>foo{"foo >"}</div>"#,
      r#"<div>foo{"foo }"}</div>"#,
      r#"<div>foo{"foo {"}</div>"#,
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
