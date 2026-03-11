// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::{ContentEq, GetSpan, Span};
use derive_more::Display;

#[derive(Debug)]
pub struct NoSelfAssign;

const CODE: &str = "no-self-assign";

#[derive(Display)]
enum NoSelfAssignMessage {
  #[display(fmt = "`{}` is assigned to itself", _0)]
  Invalid(String),
}

#[derive(Display)]
enum NoSelfAssignHint {
  #[display(
    fmt = "Self assignments have no effect. Perhaps you made a mistake?"
  )]
  Mistake,
}

impl LintRule for NoSelfAssign {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoSelfAssignVisitor;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoSelfAssignVisitor;

impl NoSelfAssignVisitor {
  fn add_diagnostic(
    &mut self,
    range: Span,
    name: impl ToString,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      NoSelfAssignMessage::Invalid(name.to_string()),
      NoSelfAssignHint::Mistake,
    );
  }

  /// Check if two member expressions refer to the same property chain.
  fn are_same_member(
    &self,
    left: &MemberExpression,
    right: &MemberExpression,
  ) -> bool {
    if !self.are_same_property(left, right) {
      return false;
    }

    let l_obj = left.object();
    let r_obj = right.object();

    match (l_obj.as_member_expression(), r_obj.as_member_expression()) {
      (Some(l_mem), Some(r_mem)) => self.are_same_member(l_mem, r_mem),
      (None, None) => match (l_obj, r_obj) {
        (Expression::ThisExpression(_), Expression::ThisExpression(_)) => true,
        (Expression::Identifier(l), Expression::Identifier(r)) => {
          l.name == r.name
        }
        _ => false,
      },
      _ => false,
    }
  }

  /// Check if two member expressions access the same property.
  fn are_same_property(
    &self,
    left: &MemberExpression,
    right: &MemberExpression,
  ) -> bool {
    match (left, right) {
      (
        MemberExpression::StaticMemberExpression(l),
        MemberExpression::StaticMemberExpression(r),
      ) => l.property.name == r.property.name,
      (
        MemberExpression::PrivateFieldExpression(l),
        MemberExpression::PrivateFieldExpression(r),
      ) => l.field.name == r.field.name,
      (
        MemberExpression::ComputedMemberExpression(l),
        MemberExpression::ComputedMemberExpression(r),
      ) => same_computed_key(&l.expression, &r.expression),
      // Static vs computed: compare by name
      (
        MemberExpression::StaticMemberExpression(s),
        MemberExpression::ComputedMemberExpression(c),
      ) => match_static_computed(s.property.name.as_str(), &c.expression),
      (
        MemberExpression::ComputedMemberExpression(c),
        MemberExpression::StaticMemberExpression(s),
      ) => match_static_computed(s.property.name.as_str(), &c.expression),
      _ => false,
    }
  }

  fn check_same_member(
    &mut self,
    left: &MemberExpression,
    right: &MemberExpression,
    ctx: &mut Context,
  ) {
    if self.are_same_member(left, right) {
      let name = member_prop_name(right, ctx);
      self.add_diagnostic(right.span(), name, ctx);
    }
  }

  fn check_target_and_expr(
    &mut self,
    left: &AssignmentTarget,
    right: &Expression,
    ctx: &mut Context,
  ) {
    match left {
      AssignmentTarget::AssignmentTargetIdentifier(l_ident) => {
        if let Expression::Identifier(r_ident) = right {
          if l_ident.name == r_ident.name {
            self.add_diagnostic(
              r_ident.span,
              r_ident.name.to_string(),
              ctx,
            );
          }
        }
      }
      AssignmentTarget::ArrayAssignmentTarget(l_arr) => {
        if let Expression::ArrayExpression(r_arr) = right {
          let end = std::cmp::min(l_arr.elements.len(), r_arr.elements.len());
          for i in 0..end {
            let left_elem = &l_arr.elements[i];
            let right_elem = &r_arr.elements[i];

            if let (Some(l_el), right_el) = (left_elem, right_elem) {
              match right_el {
                ArrayExpressionElement::SpreadElement(_spread) => {
                  // Rest elements are handled separately via l_arr.rest
                  break;
                }
                ArrayExpressionElement::Elision(_) => {}
                _ => {
                  if let AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(_) = l_el {
                    // [a = 1] = [a] - skip, default means different
                    continue;
                  }
                  if let Some(target) = l_el.as_assignment_target() {
                    if let Some(expr) = right_el.as_expression() {
                      self.check_target_and_expr(target, expr, ctx);
                    }
                  }
                }
              }
            }
          }

          // Handle rest element: [...a] = [...a]
          if let Some(rest) = &l_arr.rest {
            if let Some(ArrayExpressionElement::SpreadElement(spread)) = r_arr.elements.last() {
              if l_arr.elements.len() < r_arr.elements.len() - 1 {
                // [...a] = [...a, 1] - skip (more non-spread elements on right)
              } else {
                self.check_target_and_expr(
                  &rest.target,
                  &spread.argument,
                  ctx,
                );
              }
            }
          }
        }
      }
      AssignmentTarget::ObjectAssignmentTarget(l_obj) => {
        if let Expression::ObjectExpression(r_obj) = right {
          if r_obj.properties.is_empty() {
            return;
          }
          // Find start_j: skip past last spread
          let mut start_j = 0;
          for (index, prop) in r_obj.properties.iter().enumerate().rev() {
            if let ObjectPropertyKind::SpreadProperty(_) = prop {
              start_j = index + 1;
              break;
            }
          }

          for l_prop in &l_obj.properties {
            for j in start_j..r_obj.properties.len() {
              self.check_obj_target_prop_and_obj_prop(
                l_prop,
                &r_obj.properties[j],
                ctx,
              );
            }
          }
        }
      }
      // For member expression targets, compare with right side
      _ => {
        if let Some(l_mem) = left.as_member_expression() {
          if let Some(r_mem) = right.as_member_expression() {
            self.check_same_member(l_mem, r_mem, ctx);
          }
        }
      }
    }
  }

  fn check_obj_target_prop_and_obj_prop(
    &mut self,
    left: &AssignmentTargetProperty,
    right: &ObjectPropertyKind,
    ctx: &mut Context,
  ) {
    match (left, right) {
      (
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(l_ident),
        ObjectPropertyKind::ObjectProperty(r_prop),
      ) => {
        // shorthand: {a} = {a}
        if r_prop.shorthand {
          if let Expression::Identifier(r_ident) = &r_prop.value {
            if l_ident.init.is_none()
              && l_ident.binding.name == r_ident.name
            {
              self.add_diagnostic(
                r_ident.span,
                r_ident.name.to_string(),
                ctx,
              );
            }
          }
        } else {
          // {a} on left, {a: expr} on right
          let r_key_name = property_key_static_name(&r_prop.key);
          if let Some(rname) = r_key_name {
            if l_ident.binding.name.as_str() == rname
              && l_ident.init.is_none()
            {
              if let Expression::Identifier(r_ident) = &r_prop.value {
                if l_ident.binding.name == r_ident.name {
                  self.add_diagnostic(
                    r_ident.span,
                    r_ident.name.to_string(),
                    ctx,
                  );
                }
              }
            }
          }
        }
      }
      (
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(l_kv),
        ObjectPropertyKind::ObjectProperty(r_prop),
      ) => {
        let l_key_name = property_key_static_name(&l_kv.name);
        let r_key_name = property_key_static_name(&r_prop.key);

        if let (Some(lname), Some(rname)) = (l_key_name, r_key_name) {
          if lname == rname {
            if let AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
              _,
            ) = &l_kv.binding
            {
              // skip - has default
            } else if let Some(target) =
              l_kv.binding.as_assignment_target()
            {
              self.check_target_and_expr(target, &r_prop.value, ctx);
            }
          }
        }
      }
      _ => {}
    }
  }
}

fn member_prop_name(member: &MemberExpression, ctx: &Context) -> String {
  match member {
    MemberExpression::StaticMemberExpression(s) => {
      s.property.name.to_string()
    }
    MemberExpression::PrivateFieldExpression(p) => p.field.name.to_string(),
    MemberExpression::ComputedMemberExpression(c) => {
      match &c.expression {
        Expression::StringLiteral(s) => s.value.to_string(),
        Expression::NumericLiteral(n) => n.value.to_string(),
        Expression::BigIntLiteral(b) => {
          b.raw.as_ref().map(|r| r.to_string()).unwrap_or_default()
        }
        _ => {
          let src = ctx.source_text();
          let span = c.expression.span();
          src[span.start as usize..span.end as usize].to_string()
        }
      }
    }
  }
}

/// Check if a static property name matches a computed expression value.
fn match_static_computed(static_name: &str, computed_expr: &Expression) -> bool {
  match computed_expr {
    Expression::StringLiteral(s) => s.value.as_str() == static_name,
    Expression::TemplateLiteral(t) => {
      if t.expressions.is_empty() && t.quasis.len() == 1 {
        if let Some(cooked) = &t.quasis[0].value.cooked {
          return cooked.as_str() == static_name;
        }
      }
      false
    }
    Expression::NumericLiteral(n) => n.value.to_string() == static_name,
    _ => false,
  }
}

/// Extract a static name from a property key (identifier, string, or number).
fn property_key_static_name(key: &PropertyKey) -> Option<String> {
  match key {
    PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
    PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
    PropertyKey::NumericLiteral(n) => Some(n.value.to_string()),
    PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
    PropertyKey::TemplateLiteral(t) => {
      if t.expressions.is_empty() && t.quasis.len() == 1 {
        t.quasis[0].value.cooked.as_ref().map(|c| c.to_string())
      } else {
        None
      }
    }
    _ => None,
  }
}

impl Handler<'_> for NoSelfAssignVisitor {
  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    if assign_expr.operator != AssignmentOperator::Assign {
      return;
    }

    // Try member expression comparison first
    match (
      assign_expr.left.as_member_expression(),
      assign_expr.right.as_member_expression(),
    ) {
      (Some(l_mem), Some(r_mem)) => {
        self.check_same_member(l_mem, r_mem, ctx);
      }
      _ => {
        // Fall through to destructuring / identifier comparison
        self.check_target_and_expr(
          &assign_expr.left,
          &assign_expr.right,
          ctx,
        );
      }
    }
  }
}

/// Returns true if the computed key expression is "simple" (a literal or
/// identifier), meaning it's safe to compare two such keys for self-assign.
/// Complex expressions like `b + 1` could have side effects and we can't
/// assume both sides evaluate to the same value.
fn is_simple_computed_key(expr: &Expression) -> bool {
  matches!(
    expr,
    Expression::StringLiteral(_)
      | Expression::NumericLiteral(_)
      | Expression::BooleanLiteral(_)
      | Expression::NullLiteral(_)
      | Expression::BigIntLiteral(_)
      | Expression::RegExpLiteral(_)
      | Expression::Identifier(_)
  )
}

/// Returns the string representation of a regex literal as a property key.
/// Since `obj[/foo/]` === `obj['/foo/']`, the regex converts to `/<pattern>/<flags>`.
fn regex_as_key_string(r: &deno_ast::oxc::ast::ast::RegExpLiteral) -> String {
  format!("/{}/{}", r.regex.pattern.text, r.regex.flags)
}

/// Returns true if two computed key expressions refer to the same key,
/// accounting for the fact that regex literals coerce to strings as keys.
fn same_computed_key(l: &Expression, r: &Expression) -> bool {
  if !is_simple_computed_key(l) || !is_simple_computed_key(r) {
    return false;
  }
  // If both are exactly the same expression type and value, they match
  if l.content_eq(r) {
    return true;
  }
  // Handle regex vs string coercion: `/foo/` as a key equals `"/foo/"`
  match (l, r) {
    (Expression::RegExpLiteral(rl), Expression::StringLiteral(rs)) => {
      regex_as_key_string(rl) == rs.value.as_str()
    }
    (Expression::StringLiteral(ls), Expression::RegExpLiteral(rr)) => {
      ls.value.as_str() == regex_as_key_string(rr)
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_self_assign_valid() {
    assert_lint_ok! {
      NoSelfAssign,
      "var a = a;",
      "a = b",
      "a += a",
      "a = +a",
      "a = [a]",
      "let a = a",
      "const a = a",
      "[a] = a",
      "[a = 1] = [a]",
      "[a, b] = [b, a]",
      "[a, , b] = [, b, a]",
      "[b, a] = [...b, a]",
      "[...a] = [...a, 1]",
      "[a, ...b] = [0, ...b, 1]",
      "[a, b] = {a, b}",
      "({a} = a)",
      "({a = 1} = {a})",
      "({a: b} = {a})",
      "({a} = {a: b})",
      "({a} = {[a]: a})",
      "({[a]: b} = {[a]: b})",
      "({'foo': a, 1: a} = {'bar': a, 2: a})",
      "({a, ...b} = {a, ...b})",
      "a.b = a.c",
      "a.b = c.b",
      "a.b = a[b]",
      "a[b] = a.b",
      "a.b().c = a.b().c",
      "b().c = b().c",
      "a.null = a[/(?<zero>0)/]",
      "a[b + 1] = a[b + 1]",
      "this.x = this.y",
      "this.x = options.x",
      "this.name = this.constructor.name",

      // https://github.com/denoland/deno_lint/issues/1081
      r#"
        class Foo {
          constructor() {
            this.#bar = this.bar
          }
        }
      "#,
    };
  }

  #[test]
  fn no_self_assign_invalid() {
    assert_lint_err! {
      NoSelfAssign,
      "a = a": [
        {
          col: 4,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[a] = [a]": [
        {
          col: 7,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[a, b] = [a, b]": [
        {
          col: 10,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 13,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[a, b] = [a, c]": [
        {
          col: 10,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[a, b] = [, b]": [
        {
          col: 12,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[a, ...b] = [a, ...b]": [
        {
          col: 13,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 19,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "[[a], {b}] = [[a], {b}]": [
        {
          col: 15,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 20,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a} = {a})": [
        {
          col: 8,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a: b} = {a: b})": [
        {
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({'a': b} = {'a': b})": [
        {
          col: 18,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a: b} = {'a': b})": [
        {
          col: 16,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({'a': b} = {a: b})": [
        {
          col: 16,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({1: b} = {1: b})": [
        {
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({1: b} = {'1': b})": [
        {
          col: 16,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({'1': b} = {1: b})": [
        {
          col: 16,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({['a']: b} = {a: b})": [
        {
          col: 18,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({'a': b} = {[`a`]: b})": [
        {
          col: 20,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({1: b} = {[1]: b})": [
        {
          col: 16,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a, b} = {a, b})": [
        {
          col: 11,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a, b} = {b, a})": [
        {
          col: 11,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a, b} = {c, a})": [
        {
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a: {b}, c: [d]} = {a: {b}, c: [d]})": [
        {
          col: 25,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 33,
          message: variant!(NoSelfAssignMessage, Invalid, "d"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "({a, b} = {a, ...x, b})": [
        {
          col: 20,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a.b = a.b": [
        {
          col: 6,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a.b.c = a.b.c": [
        {
          col: 8,
          message: variant!(NoSelfAssignMessage, Invalid, "c"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a[b] = a[b]": [
        {
          col: 7,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a['b'] = a['b']": [
        {
          col: 9,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a[\n    'b'] = a[\n    'b']": [
        {
          line: 2,
          col: 11,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "this.x = this.x": [
        {
          col: 9,
          message: variant!(NoSelfAssignMessage, Invalid, "x"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
      "a['/(?<zero>0)/'] = a[/(?<zero>0)/]": [
        {
          col: 20,
          message: variant!(NoSelfAssignMessage, Invalid, "/(?<zero>0)/"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],

      // check if it works for nested assignments
      "foo = () => { a = a; };": [
        {
          col: 18,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],

      r#"
        class Foo {
          constructor() {
            this.#bar = this.#bar
          }
        }
      "#: [
        {
          line: 4,
          col: 24,
          message: variant!(NoSelfAssignMessage, Invalid, "bar"),
          hint: NoSelfAssignHint::Mistake,
        }
      ],
    };
  }
}
