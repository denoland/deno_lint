// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElementName, JSXExpr,
  Lit, Tpl,
};
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct ButtonHasType;

const CODE: &str = "button-has-type";

impl LintRule for ButtonHasType {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX, tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    HasButtonTypeHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/button_has_type.md")
  }
}

enum DiagnosticKind {
  MissingTypeAttr,
  MissingValue,
  WrongValue,
}

impl DiagnosticKind {
  #[cfg(test)]
  fn message_and_hint(&self) -> (&'static str, &'static str) {
    (self.message(), self.hint())
  }

  fn message(&self) -> &'static str {
    match *self {
      DiagnosticKind::MissingTypeAttr => {
        "`button` elements must have a `type` attribute"
      }
      DiagnosticKind::MissingValue => {
        "Missing value for button's `type` attribute"
      }
      DiagnosticKind::WrongValue => {
        "Incorrect value for button's `type` attribute"
      }
    }
  }

  fn hint(&self) -> &'static str {
    match *self {
      DiagnosticKind::MissingTypeAttr => {
        "Add a `type` attribute with a value of `button`, `submit` or `reset`"
      }
      DiagnosticKind::MissingValue | DiagnosticKind::WrongValue => "The value of the `type` attribute must be one of `button`, `submit` or `reset`",
    }
  }
}

struct HasButtonTypeHandler;

impl Handler for HasButtonTypeHandler {
  fn jsx_opening_element(
    &mut self,
    node: &ast_view::JSXOpeningElement,
    ctx: &mut Context,
  ) {
    if let JSXElementName::Ident(id) = node.name {
      if id.sym() != "button" {
        return;
      }

      let mut found = false;
      for attr in node.attrs {
        let JSXAttrOrSpread::JSXAttr(attr) = attr else {
          continue;
        };

        let JSXAttrName::Ident(name) = attr.name else {
          continue;
        };

        if name.sym() == "type" {
          found = true;

          if let Some(attr_value) = attr.value {
            let kind = DiagnosticKind::WrongValue;
            match attr_value {
              JSXAttrValue::Lit(lit) => {
                if let Lit::Str(lit_str) = lit {
                  let value = lit_str.value();
                  if !is_valid_value(value) {
                    ctx.add_diagnostic_with_hint(
                      attr_value.range(),
                      CODE,
                      kind.message(),
                      kind.hint(),
                    );
                  }
                } else {
                  ctx.add_diagnostic_with_hint(
                    attr_value.range(),
                    CODE,
                    kind.message(),
                    kind.hint(),
                  );
                }
              }
              JSXAttrValue::JSXExprContainer(expr) => {
                let JSXExpr::Expr(expr) = expr.expr else {
                  continue;
                };

                match expr {
                  Expr::Cond(cond_expr) => {
                    match cond_expr.cons {
                      Expr::Lit(lit) => {
                        check_literal_value(ctx, &lit);
                      }
                      Expr::Tpl(tpl) => check_tpl(ctx, tpl),
                      _ => ctx.add_diagnostic_with_hint(
                        cond_expr.cons.range(),
                        CODE,
                        kind.message(),
                        kind.hint(),
                      ),
                    };

                    match cond_expr.alt {
                      Expr::Lit(lit) => {
                        check_literal_value(ctx, &lit);
                      }
                      Expr::Tpl(tpl) => check_tpl(ctx, tpl),
                      _ => ctx.add_diagnostic_with_hint(
                        cond_expr.alt.range(),
                        CODE,
                        kind.message(),
                        kind.hint(),
                      ),
                    };
                  }
                  Expr::Lit(lit) => {
                    check_literal_value(ctx, &lit);
                  }
                  Expr::Tpl(tpl) => check_tpl(ctx, tpl),
                  _ => {
                    ctx.add_diagnostic_with_hint(
                      attr_value.range(),
                      CODE,
                      kind.message(),
                      kind.hint(),
                    );
                  }
                }
              }
              _ => {
                let kind = DiagnosticKind::WrongValue;
                ctx.add_diagnostic_with_hint(
                  attr_value.range(),
                  CODE,
                  kind.message(),
                  kind.hint(),
                );
              }
            }
          } else {
            let kind = DiagnosticKind::MissingValue;
            ctx.add_diagnostic_with_hint(
              attr.name.range(),
              CODE,
              kind.message(),
              kind.hint(),
            );
            return;
          };
        }
      }

      if !found {
        let kind = DiagnosticKind::MissingTypeAttr;
        ctx.add_diagnostic_with_hint(
          node.range(),
          CODE,
          kind.message(),
          kind.hint(),
        );
      }
    }
  }
}

fn is_valid_value(value: &str) -> bool {
  value == "submit" || value == "button" || value == "reset"
}

fn check_tpl(ctx: &mut Context, tpl: &Tpl<'_>) {
  let kind = DiagnosticKind::WrongValue;
  if !tpl.exprs.is_empty() {
    ctx.add_diagnostic_with_hint(
      tpl.range(),
      CODE,
      kind.message(),
      kind.hint(),
    );
    return;
  }

  if let Some(first) = tpl.quasis.first() {
    if !is_valid_value(first.inner.raw.as_str()) {
      ctx.add_diagnostic_with_hint(
        tpl.range(),
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
  }
}

fn check_literal_value(ctx: &mut Context, lit: &Lit) {
  let kind = DiagnosticKind::WrongValue;

  match lit {
    Lit::Str(lit_str) => {
      let value = lit_str.value();
      if !is_valid_value(value) {
        ctx.add_diagnostic_with_hint(
          lit.range(),
          CODE,
          kind.message(),
          kind.hint(),
        );
      }
    }
    _ => ctx.add_diagnostic_with_hint(
      lit.range(),
      CODE,
      kind.message(),
      kind.hint(),
    ),
  }
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn button_has_type_valid() {
    assert_lint_ok! {
      ButtonHasType,
      filename: "file:///foo.jsx",
      // non derived classes.
      r#"<button type="button" />"#,
      r#"<button type="submit" />"#,
      r#"<button type="reset" />"#,
      r#"<button type={"button"} />"#,
      r#"<button type={"submit"} />"#,
      r#"<button type={"reset"} />"#,
      r#"<button type={`reset`} />"#,
      r#"<button type={condition ? "submit" : "button"} />"#,
      r#"<button type={condition ? 'submit' : 'button'} />"#,
      r#"<button type={condition ? `submit` : `button`} />"#,
    };
  }

  #[test]
  fn button_has_type_invalid() {
    let (wrong_value_message, wrong_value_hint) =
      DiagnosticKind::WrongValue.message_and_hint();
    let (missing_type_message, missing_type_hint) =
      DiagnosticKind::MissingTypeAttr.message_and_hint();
    let (missing_value_message, missing_value_hint) =
      DiagnosticKind::MissingValue.message_and_hint();

    assert_lint_err! {
      ButtonHasType,
      filename: "file:///foo.jsx",
      "<button />": [
        {
          col: 0,
          message: missing_type_message,
          hint: missing_type_hint,
        }
      ],
      "<button type />": [
        {
          col: 8,
          message: missing_value_message,
          hint: missing_value_hint,
        }
      ],
      "<button type='foo' />": [
        {
          col: 13,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={foo} />": [
        {
          col: 13,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={'foo'} />": [
        {
          col: 14,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={2} />": [
        {
          col: 14,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={condition ? foo : 'button'} />": [
        {
          col: 26,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={condition ? 'button' : foo} />": [
        {
          col: 37,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={condition ? `foo` : `button`} />": [
        {
          col: 26,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ],
      "<button type={condition ? `button` : `foo`} />": [
        {
          col: 37,
          message: wrong_value_message,
          hint: wrong_value_hint,
        }
      ]
    };
  }
}
