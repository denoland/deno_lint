// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Expression, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
  JSXElementName, JSXExpression, JSXOpeningElement, Program, TemplateLiteral,
};
use deno_ast::oxc::span::GetSpan;

#[derive(Debug)]
pub struct JSXButtonHasType;

const CODE: &str = "jsx-button-has-type";

impl LintRule for JSXButtonHasType {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX, tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = HasButtonTypeHandler;
    crate::handler::traverse_program(&mut handler, program, context);
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

impl Handler<'_> for HasButtonTypeHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    let is_button = match &node.name {
      JSXElementName::Identifier(id) => id.name.as_str() == "button",
      _ => false,
    };
    if !is_button {
      return;
    }

    let mut found = false;
    let mut has_spread = false;
    for attr in &node.attributes {
      let JSXAttributeItem::Attribute(attr) = attr else {
        has_spread = true;
        continue;
      };

      let JSXAttributeName::Identifier(name) = &attr.name else {
        continue;
      };

      if name.name.as_str() == "type" {
        found = true;

        if let Some(attr_value) = &attr.value {
          let kind = DiagnosticKind::WrongValue;
          match attr_value {
            JSXAttributeValue::StringLiteral(lit_str) => {
              if !is_valid_value(lit_str.value.as_str()) {
                ctx.add_diagnostic_with_hint(
                  attr_value.span(),
                  CODE,
                  kind.message(),
                  kind.hint(),
                );
              }
            }
            JSXAttributeValue::ExpressionContainer(expr) => {
              let JSXExpression::EmptyExpression(_) = &expr.expression else {
                match &expr.expression {
                  JSXExpression::BooleanLiteral(lit) => {
                    ctx.add_diagnostic_with_hint(
                      lit.span,
                      CODE,
                      kind.message(),
                      kind.hint(),
                    );
                  }
                  JSXExpression::NullLiteral(lit) => {
                    ctx.add_diagnostic_with_hint(
                      lit.span,
                      CODE,
                      kind.message(),
                      kind.hint(),
                    );
                  }
                  JSXExpression::NumericLiteral(lit) => {
                    ctx.add_diagnostic_with_hint(
                      lit.span,
                      CODE,
                      kind.message(),
                      kind.hint(),
                    );
                  }
                  JSXExpression::StringLiteral(lit) => {
                    if !is_valid_value(lit.value.as_str()) {
                      ctx.add_diagnostic_with_hint(
                        lit.span,
                        CODE,
                        kind.message(),
                        kind.hint(),
                      );
                    }
                  }
                  JSXExpression::TemplateLiteral(tpl) => {
                    check_tpl(ctx, tpl);
                  }
                  JSXExpression::ConditionalExpression(cond_expr) => {
                    check_expression_value(ctx, &cond_expr.consequent);
                    check_expression_value(ctx, &cond_expr.alternate);
                  }
                  _ => {
                    // We can't reliably check these cases without
                    // type information. Therefore, we ignore them.
                  }
                }
                continue;
              };
            }
            _ => {
              // We can't reliably check these cases without
              // type information. Therefore, we ignore them.
            }
          }
        } else {
          let kind = DiagnosticKind::MissingValue;
          ctx.add_diagnostic_with_hint(
            attr.name.span(),
            CODE,
            kind.message(),
            kind.hint(),
          );
          return;
        };
      }
    }

    if !found && !has_spread {
      let kind = DiagnosticKind::MissingTypeAttr;
      ctx.add_diagnostic_with_hint(
        node.name.span(),
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
  }
}

fn is_valid_value(value: &str) -> bool {
  value == "submit" || value == "button" || value == "reset"
}

fn check_tpl(ctx: &mut Context, tpl: &TemplateLiteral) {
  let kind = DiagnosticKind::WrongValue;
  if !tpl.expressions.is_empty() {
    ctx.add_diagnostic_with_hint(
      tpl.span,
      CODE,
      kind.message(),
      kind.hint(),
    );
    return;
  }

  if let Some(first) = tpl.quasis.first() {
    if !is_valid_value(first.value.raw.as_str()) {
      ctx.add_diagnostic_with_hint(
        tpl.span,
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
  }
}

fn check_expression_value(ctx: &mut Context, expr: &Expression) {
  let kind = DiagnosticKind::WrongValue;

  match expr {
    Expression::StringLiteral(lit_str) => {
      if !is_valid_value(lit_str.value.as_str()) {
        ctx.add_diagnostic_with_hint(
          lit_str.span,
          CODE,
          kind.message(),
          kind.hint(),
        );
      }
    }
    Expression::TemplateLiteral(tpl) => check_tpl(ctx, tpl),
    Expression::BooleanLiteral(lit) => {
      ctx.add_diagnostic_with_hint(
        lit.span,
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    Expression::NullLiteral(lit) => {
      ctx.add_diagnostic_with_hint(
        lit.span,
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    Expression::NumericLiteral(lit) => {
      ctx.add_diagnostic_with_hint(
        lit.span,
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    _ => {
      ctx.add_diagnostic_with_hint(
        expr.span(),
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
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
      JSXButtonHasType,
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
      r#"<button type={foo} />"#,
      r#"<button type={foo()} />"#,
      r#"<button {...props} />"#,
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
      JSXButtonHasType,
      filename: "file:///foo.jsx",
      "<button />": [
        {
          col: 1,
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
