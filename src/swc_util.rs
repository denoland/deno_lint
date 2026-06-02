// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::oxc::ast::ast::{
  Argument, BooleanLiteral, Expression, IdentifierName, IdentifierReference,
  MemberExpression, NumericLiteral, ObjectProperty, ObjectPropertyKind,
  PropertyKey, RegExpLiteral, StringLiteral, TemplateLiteral,
};
use deno_ast::oxc::semantic::Scoping;

/// Extracts regex string from an expression, using OXC scoping.
/// If the passed expression is not a `RegExp` call on the global, returns `None`.
pub(crate) fn extract_regex(
  scoping: &Scoping,
  expr_ident: &IdentifierReference,
  expr_args: &[Argument],
) -> Option<String> {
  if expr_ident.name.as_str() != "RegExp" {
    return None;
  }

  // Check if the identifier resolves to a local binding via OXC scoping.
  if let Some(ref_id) = expr_ident.reference_id.get() {
    let reference = scoping.get_reference(ref_id);
    if reference.symbol_id().is_some() {
      return None; // shadowed by a local binding
    }
  }

  match expr_args.first() {
    Some(Argument::StringLiteral(literal)) => Some(literal.value.to_string()),
    Some(Argument::RegExpLiteral(regex)) => {
      Some(regex.regex.pattern.text.to_string())
    }
    _ => None,
  }
}

pub(crate) trait StringRepr {
  fn string_repr(&self) -> Option<String>;
}

impl StringRepr for StringLiteral<'_> {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string())
  }
}

impl StringRepr for BooleanLiteral {
  fn string_repr(&self) -> Option<String> {
    let s = if self.value { "true" } else { "false" };
    Some(s.to_string())
  }
}

impl StringRepr for NumericLiteral<'_> {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string())
  }
}

impl StringRepr for RegExpLiteral<'_> {
  fn string_repr(&self) -> Option<String> {
    Some(format!("/{}/", self.regex.pattern.text))
  }
}

impl StringRepr for IdentifierReference<'_> {
  fn string_repr(&self) -> Option<String> {
    Some(self.name.to_string())
  }
}

impl StringRepr for IdentifierName<'_> {
  fn string_repr(&self) -> Option<String> {
    Some(self.name.to_string())
  }
}

impl StringRepr for ObjectPropertyKind<'_> {
  fn string_repr(&self) -> Option<String> {
    match self {
      ObjectPropertyKind::ObjectProperty(p) => p.string_repr(),
      ObjectPropertyKind::SpreadProperty(_) => None,
    }
  }
}

impl StringRepr for ObjectProperty<'_> {
  fn string_repr(&self) -> Option<String> {
    self.key.string_repr()
  }
}

impl StringRepr for Expression<'_> {
  fn string_repr(&self) -> Option<String> {
    match self {
      Expression::Identifier(ident) => ident.string_repr(),
      Expression::StringLiteral(s) => s.string_repr(),
      Expression::BooleanLiteral(b) => b.string_repr(),
      Expression::NumericLiteral(n) => n.string_repr(),
      Expression::BigIntLiteral(b) => Some(b.value.to_string()),
      Expression::RegExpLiteral(r) => r.string_repr(),
      Expression::TemplateLiteral(tpl) => tpl.string_repr(),
      Expression::NullLiteral(_) => Some("null".to_string()),
      _ => None,
    }
  }
}

impl StringRepr for PropertyKey<'_> {
  fn string_repr(&self) -> Option<String> {
    match self {
      PropertyKey::StaticIdentifier(i) => i.string_repr(),
      PropertyKey::StringLiteral(s) => s.string_repr(),
      PropertyKey::NumericLiteral(n) => n.string_repr(),
      _ => {
        // For computed property keys, try to extract from the expression
        if let Some(expr) = self.as_expression() {
          match expr {
            Expression::StringLiteral(s) => s.string_repr(),
            Expression::TemplateLiteral(tpl) => tpl.string_repr(),
            Expression::NumericLiteral(n) => n.string_repr(),
            Expression::BigIntLiteral(b) => Some(b.value.to_string()),
            Expression::NullLiteral(_) => Some("null".to_string()),
            Expression::RegExpLiteral(r) => r.string_repr(),
            _ => None,
          }
        } else {
          None
        }
      }
    }
  }
}

impl StringRepr for TemplateLiteral<'_> {
  fn string_repr(&self) -> Option<String> {
    if self.expressions.is_empty() {
      self.quasis.first().map(|q| q.value.raw.to_string())
    } else {
      None
    }
  }
}

impl StringRepr for MemberExpression<'_> {
  fn string_repr(&self) -> Option<String> {
    match self {
      MemberExpression::StaticMemberExpression(s) => s.property.string_repr(),
      MemberExpression::PrivateFieldExpression(p) => {
        Some(p.field.name.to_string())
      }
      MemberExpression::ComputedMemberExpression(_) => None,
    }
  }
}

impl<S: StringRepr> StringRepr for Option<S> {
  fn string_repr(&self) -> Option<String> {
    self.as_ref().and_then(|k| k.string_repr())
  }
}
