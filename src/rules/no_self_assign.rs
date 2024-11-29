// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use crate::Program;

use deno_ast::view::AssignExpr;
use deno_ast::view::AssignOp;
use deno_ast::view::AssignTarget;
use deno_ast::view::Expr;
use deno_ast::view::ExprOrSpread;
use deno_ast::view::Ident;
use deno_ast::view::MemberExpr;
use deno_ast::view::MemberProp;
use deno_ast::view::ObjectPatProp;
use deno_ast::view::Pat;
use deno_ast::view::Prop;
use deno_ast::view::PropOrSpread;
use deno_ast::{SourceRange, SourceRanged};
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

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    NoSelfAssignVisitor.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_self_assign.md")
  }
}

struct NoSelfAssignVisitor;

impl NoSelfAssignVisitor {
  fn add_diagnostic(
    &mut self,
    range: SourceRange,
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

  fn are_same_property(
    &mut self,
    left: &MemberExpr,
    right: &MemberExpr,
  ) -> bool {
    match (&left.prop, &right.prop) {
      (MemberProp::Ident(_), MemberProp::PrivateName(_)) => {
        return false;
      }
      (MemberProp::PrivateName(_), MemberProp::Ident(_)) => {
        return false;
      }
      _ => {}
    }

    if let (
      MemberProp::Computed(l_computed),
      MemberProp::Computed(r_computed),
    ) = (&left.prop, &right.prop)
    {
      match (l_computed.expr, r_computed.expr) {
        (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
          if self.are_same_ident(l_ident, r_ident) {
            return true;
          }
        }
        (Expr::Lit(l_lit), Expr::Lit(r_lit)) => {
          if l_lit.string_repr() == r_lit.string_repr() {
            return true;
          }
        }
        _ => {}
      }
    }

    let left_name = if matches!(left.prop, MemberProp::Computed(_)) {
      None
    } else {
      left.string_repr()
    };
    let right_name = if matches!(right.prop, MemberProp::Computed(_)) {
      None
    } else {
      right.string_repr()
    };

    if let Some(lname) = left_name {
      if let Some(rname) = right_name {
        return lname == rname;
      }
    }

    false
  }

  fn are_same_member(&mut self, left: &MemberExpr, right: &MemberExpr) -> bool {
    let same_prop = self.are_same_property(left, right);
    if !same_prop {
      return false;
    }

    match (left.obj, right.obj) {
      (Expr::Member(l_member_expr), Expr::Member(r_member_expr)) => {
        self.are_same_member(l_member_expr, r_member_expr)
      }
      (Expr::This(_), Expr::This(_)) => true,
      (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.are_same_ident(l_ident, r_ident)
      }
      _ => false,
    }
  }

  fn check_same_member(
    &mut self,
    left: &MemberExpr,
    right: &MemberExpr,
    ctx: &mut Context,
  ) {
    if self.are_same_member(left, right) {
      let name = match &right.prop {
        MemberProp::Ident(ident) => ident.string_repr(),
        MemberProp::Computed(computed) => computed.expr.string_repr(),
        MemberProp::PrivateName(name) => name.string_repr(),
      }
      .expect("Should be identifier");
      self.add_diagnostic(right.range(), name, ctx);
    }
  }

  fn are_same_ident(&mut self, left: &Ident, right: &Ident) -> bool {
    left.sym() == right.sym()
  }

  fn check_same_ident(
    &mut self,
    left: &Ident,
    right: &Ident,
    ctx: &mut Context,
  ) {
    if self.are_same_ident(left, right) {
      self.add_diagnostic(right.range(), right.sym(), ctx);
    }
  }

  fn check_expr_and_expr(
    &mut self,
    left: Expr,
    right: Expr,
    ctx: &mut Context,
  ) {
    match (left, right) {
      (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(l_ident, r_ident, ctx);
      }
      (Expr::Member(l_member), Expr::Member(r_member)) => {
        self.check_same_member(l_member, r_member, ctx);
      }
      _ => {}
    }
  }

  fn check_pat_and_spread_or_expr(
    &mut self,
    left: Pat,
    right: &ExprOrSpread,
    ctx: &mut Context,
  ) {
    if right.spread().is_some() {
      if let Pat::Rest(rest_pat) = left {
        self.check_pat_and_expr(rest_pat.arg, right.expr, ctx)
      }
    } else {
      self.check_pat_and_expr(left, right.expr, ctx);
    }
  }

  fn check_object_pat_prop_and_prop_or_spread(
    &mut self,
    left: &ObjectPatProp,
    right: &PropOrSpread,
    ctx: &mut Context,
  ) {
    match (left, right) {
      (
        ObjectPatProp::Assign(assign_pat_prop),
        PropOrSpread::Prop(Prop::Shorthand(right_ident)),
      ) => {
        if assign_pat_prop.value.is_none() {
          self.check_same_ident(assign_pat_prop.key.id, right_ident, ctx);
        }
      }
      (
        ObjectPatProp::KeyValue(key_val_pat_prop),
        PropOrSpread::Prop(Prop::KeyValue(right_prop)),
      ) => {
        let left_name = key_val_pat_prop.key.string_repr();
        let right_name = right_prop.key.string_repr();

        if let Some(lname) = left_name {
          if let Some(rname) = right_name {
            if lname == rname {
              self.check_pat_and_expr(
                key_val_pat_prop.value,
                right_prop.value,
                ctx,
              );
            }
          }
        }
      }
      _ => {}
    }
  }

  fn check_pat_and_expr(&mut self, left: Pat, right: Expr, ctx: &mut Context) {
    match (left, right) {
      (Pat::Expr(l_expr), _) => {
        self.check_expr_and_expr(l_expr, right, ctx);
      }
      (Pat::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(l_ident.id, r_ident, ctx);
      }
      (Pat::Array(l_array_pat), Expr::Array(r_array_lit)) => {
        let end =
          std::cmp::min(l_array_pat.elems.len(), r_array_lit.elems.len());
        for i in 0..end {
          let left_elem = &l_array_pat.elems[i];
          let right_elem = &r_array_lit.elems[i];
          // Avoid cases such as [...a] = [...a, 1]
          if let Some(Pat::Rest(_)) = left_elem {
            if i < r_array_lit.elems.len() - 1 {
              break;
            }
          }

          if left_elem.is_some() && right_elem.is_some() {
            self.check_pat_and_spread_or_expr(
              *left_elem.as_ref().unwrap(),
              right_elem.as_ref().unwrap(),
              ctx,
            );
          }

          if let Some(elem) = right_elem {
            if elem.spread().is_some() {
              break;
            }
          }
        }
      }
      (Pat::Object(l_obj), Expr::Object(r_obj)) => {
        if !r_obj.props.is_empty() {
          let mut start_j = 0;

          for (index, prop) in r_obj.props.iter().rev().enumerate() {
            if let PropOrSpread::Spread(_) = prop {
              start_j = index + 1;
              break;
            }
          }

          for i in 0..l_obj.props.len() {
            for j in start_j..r_obj.props.len() {
              self.check_object_pat_prop_and_prop_or_spread(
                &l_obj.props[i],
                &r_obj.props[j],
                ctx,
              )
            }
          }
        }
      }
      _ => {}
    }
  }
}

impl Handler for NoSelfAssignVisitor {
  fn assign_expr(&mut self, assign_expr: &AssignExpr, ctx: &mut Context) {
    if assign_expr.op() == AssignOp::Assign {
      match &assign_expr.left {
        AssignTarget::Simple(l_expr) => {
          self.check_expr_and_expr(l_expr.as_expr(), assign_expr.right, ctx);
        }
        AssignTarget::Pat(l_pat) => {
          self.check_pat_and_expr(l_pat.as_pat(), assign_expr.right, ctx);
        }
      }
    }
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
