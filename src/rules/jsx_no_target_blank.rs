// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{
  Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXExpr, JSXOpeningElement,
  Lit,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXNoTargetBlank;

const CODE: &str = "jsx-no-target-blank";

impl LintRule for JSXNoTargetBlank {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXNoTargetBlankHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_no_target_blank.md")
  }
}

const MESSAGE: &str = "Using 'target=\"blank\"' without 'rel=\"noopener\" can lead to security issues in older browsers, see https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes/rel/noopener";
const HINT: &str = "Add the 'rel=\"noopener\"' attribute";

#[derive(Debug, PartialEq)]
enum LiteralKind {
  Lit,
  BinCond,
  CondCons,
  CondAlt,
}

struct JSXNoTargetBlankHandler;

impl Handler for JSXNoTargetBlankHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    if let Some(blank_kind) = get_target_blank_kind(&node.attrs) {
      let mut found_rel = false;
      for attr in node.attrs {
        if let JSXAttrOrSpread::JSXAttr(attr) = attr {
          if let JSXAttrName::Ident(name) = attr.name {
            if name.sym() == "rel" {
              found_rel = true;

              let kind = if let Some(value) = attr.value {
                match value {
                  JSXAttrValue::Lit(lit) => {
                    get_rel_noopener_kind(&Expr::Lit(lit))
                  }
                  JSXAttrValue::JSXExprContainer(expr) => match expr.expr {
                    JSXExpr::JSXEmptyExpr(_) => None,
                    JSXExpr::Expr(expr) => get_rel_noopener_kind(&expr),
                  },
                  _ => None,
                }
              } else {
                None
              };

              if let Some(kind) = kind {
                if blank_kind != kind {
                  ctx.add_diagnostic_with_hint(
                    attr.range(),
                    CODE,
                    MESSAGE,
                    HINT,
                  );
                }
              } else {
                ctx.add_diagnostic_with_hint(attr.range(), CODE, MESSAGE, HINT);
              }
            }
          }
        }
      }

      if !found_rel {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
      }
    }
  }
}

fn get_target_blank_kind(attrs: &[JSXAttrOrSpread<'_>]) -> Option<LiteralKind> {
  for attr in attrs {
    if let JSXAttrOrSpread::JSXAttr(attr) = attr {
      if let JSXAttrName::Ident(name) = attr.name {
        if name.sym() == "target" {
          if let Some(attr_value) = attr.value {
            match attr_value {
              JSXAttrValue::Lit(lit) => {
                if let Lit::Str(lit_str) = lit {
                  if lit_str.value().as_str() == "_blank" {
                    return Some(LiteralKind::Lit);
                  }
                }
              }
              JSXAttrValue::JSXExprContainer(expr) => {
                if let JSXExpr::Expr(expr) = expr.expr {
                  if let Some(kind) = get_literal_kind(&expr, "_blank") {
                    return Some(kind);
                  }
                }
              }
              _ => {}
            }
          }
        }
      }
    }
  }

  None
}

fn get_literal_kind(expr: &Expr, value: &str) -> Option<LiteralKind> {
  match expr {
    Expr::Lit(lit) => {
      if let Lit::Str(lit_str) = lit {
        if lit_str.value().as_str() == value {
          return Some(LiteralKind::Lit);
        }
      }
    }
    Expr::Bin(bin) => {
      if get_literal_kind(&bin.right, value).is_some() {
        return Some(LiteralKind::BinCond);
      }
    }
    Expr::Tpl(tpl) => {
      if let Some(first) = &tpl.quasis.first() {
        if first.raw() == value {
          return Some(LiteralKind::Lit);
        }
      }
    }
    Expr::Cond(cond) => {
      if get_literal_kind(&cond.cons, value).is_some() {
        return Some(LiteralKind::CondCons);
      }

      if get_literal_kind(&cond.alt, value).is_some() {
        return Some(LiteralKind::CondAlt);
      }
    }
    _ => {}
  }

  None
}

fn get_rel_noopener_kind(expr: &Expr) -> Option<LiteralKind> {
  match expr {
    Expr::Lit(lit) => {
      if let Lit::Str(lit_str) = lit {
        let parts = lit_str.value().as_str().split_ascii_whitespace();
        for part in parts {
          if part == "noopener" {
            return Some(LiteralKind::Lit);
          }
        }
      }
    }
    Expr::Bin(bin) => {
      if get_rel_noopener_kind(&bin.right).is_some() {
        return Some(LiteralKind::BinCond);
      }
    }
    Expr::Tpl(tpl) => {
      if let Some(first) = &tpl.quasis.first() {
        let parts = first.raw().split_ascii_whitespace();
        for part in parts {
          if part == "noopener" {
            return Some(LiteralKind::Lit);
          }
        }
      }
    }
    Expr::Cond(cond) => {
      if get_rel_noopener_kind(&cond.cons).is_some() {
        return Some(LiteralKind::CondCons);
      }

      if get_rel_noopener_kind(&cond.alt).is_some() {
        return Some(LiteralKind::CondAlt);
      }
    }
    _ => {}
  }

  None
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_no_target_blank_valid() {
    assert_lint_ok! {
      JSXNoTargetBlank,
      filename: "file:///foo.jsx",
      r#"<a target="_blank" rel="noopener" />"#,
      r#"<a target={"_blank"} rel={"noopener"} />"#,
      r#"<a target={`_blank`} rel={`noopener`} />"#,
      r#"<a target="_blank" rel="noopener noreferrer" />"#,
      r#"<a target={foo && "_blank"} rel={foo && "noopener"} />"#,
      r#"<a target={foo ? "_blank" : null} rel={foo ? "noopener" : null} />"#,
      r#"<a target={foo ? `_blank` : null} rel={foo ? `noopener` : null} />"#,
      r#"<a target={foo ? `_blank` : null} rel={foo ? `noopener noreferrer` : null} />"#,
      r#"<Link target="_blank" rel="noopener" />"#,
      r#"<Link target="_blank" rel="noopener noreferrer" />"#,
    };
  }

  #[test]
  fn jsx_no_target_blank_invalid() {
    assert_lint_err! {
      JSXNoTargetBlank,
      filename: "file:///foo.jsx",
      r#"<a target="_blank" />"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<Link target="_blank" />"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a target="_blank" rel="foo" />"#: [
        {
          col: 19,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"<a target={foo ? "_blank" : null} rel={foo ? null : "noopener"} />"#: [
        {
          col: 34,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
