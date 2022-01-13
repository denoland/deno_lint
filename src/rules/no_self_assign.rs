// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_util::StringRepr;
use crate::ProgramRef;
use std::sync::Arc;

use deno_ast::swc::ast::AssignExpr;
use deno_ast::swc::ast::AssignOp;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ExprOrSpread;
use deno_ast::swc::ast::Ident;
use deno_ast::swc::ast::MemberExpr;
use deno_ast::swc::ast::MemberProp;
use deno_ast::swc::ast::ObjectPatProp;
use deno_ast::swc::ast::Pat;
use deno_ast::swc::ast::PatOrExpr;
use deno_ast::swc::ast::Prop;
use deno_ast::swc::ast::PropOrSpread;
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::{VisitAll, VisitAllWith};
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
    fmt = "Self assignments have no effect. Perhaps you make a mistake?"
  )]
  Mistake,
}

impl LintRule for NoSelfAssign {
  fn new() -> Arc<Self> {
    Arc::new(NoSelfAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoSelfAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_self_assign.md")
  }
}

struct NoSelfAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoSelfAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, name: impl ToString) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoSelfAssignMessage::Invalid(name.to_string()),
      NoSelfAssignHint::Mistake,
    );
  }

  fn is_same_property(
    &mut self,
    left: &MemberExpr,
    right: &MemberExpr,
  ) -> bool {
    if let (
      MemberProp::Computed(l_computed),
      MemberProp::Computed(r_computed),
    ) = (&left.prop, &right.prop)
    {
      match (&*l_computed.expr, &*r_computed.expr) {
        (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
          if self.is_same_ident(l_ident, r_ident) {
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

  fn is_same_member(&mut self, left: &MemberExpr, right: &MemberExpr) -> bool {
    let same_prop = self.is_same_property(left, right);
    if !same_prop {
      return false;
    }

    match (&*left.obj, &*right.obj) {
      (Expr::Member(l_member_expr), Expr::Member(r_member_expr)) => {
        self.is_same_member(l_member_expr, r_member_expr)
      }
      (Expr::This(_), Expr::This(_)) => true,
      (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.is_same_ident(l_ident, r_ident)
      }
      _ => false,
    }
  }

  fn check_same_member(&mut self, left: &MemberExpr, right: &MemberExpr) {
    if self.is_same_member(left, right) {
      let name = match &right.prop {
        MemberProp::Ident(ident) => ident.string_repr(),
        MemberProp::Computed(computed) => computed.expr.string_repr(),
        MemberProp::PrivateName(name) => name.string_repr(),
      }
      .expect("Should be identifier");
      self.add_diagnostic(right.span, name);
    }
  }

  fn is_same_ident(&mut self, left: &Ident, right: &Ident) -> bool {
    left.sym == right.sym
  }

  fn check_same_ident(&mut self, left: &Ident, right: &Ident) {
    if self.is_same_ident(left, right) {
      self.add_diagnostic(right.span, &right.sym);
    }
  }

  fn check_expr_and_expr(&mut self, left: &Expr, right: &Expr) {
    match (left, right) {
      (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(l_ident, r_ident);
      }
      (Expr::Member(l_member), Expr::Member(r_member)) => {
        self.check_same_member(l_member, r_member);
      }
      _ => {}
    }
  }

  fn check_pat_and_spread_or_expr(&mut self, left: &Pat, right: &ExprOrSpread) {
    if right.spread.is_some() {
      if let Pat::Rest(rest_pat) = left {
        self.check_pat_and_expr(&*rest_pat.arg, &*right.expr)
      }
    } else {
      self.check_pat_and_expr(left, &*right.expr);
    }
  }

  fn check_object_pat_prop_and_prop_or_spread(
    &mut self,
    left: &ObjectPatProp,
    right: &PropOrSpread,
  ) {
    match (left, right) {
      (
        ObjectPatProp::Assign(assign_pat_prop),
        PropOrSpread::Prop(boxed_prop),
      ) => {
        if let Prop::Shorthand(ident) = &**boxed_prop {
          if assign_pat_prop.value.is_none() {
            self.check_same_ident(&assign_pat_prop.key, ident);
          }
        }
      }
      (
        ObjectPatProp::KeyValue(key_val_pat_prop),
        PropOrSpread::Prop(boxed_prop),
      ) => {
        if let Prop::KeyValue(key_value_prop) = &**boxed_prop {
          let left_name = (&key_val_pat_prop.key).string_repr();
          let right_name = (&key_value_prop.key).string_repr();

          if let Some(lname) = left_name {
            if let Some(rname) = right_name {
              if lname == rname {
                self.check_pat_and_expr(
                  &*key_val_pat_prop.value,
                  &*key_value_prop.value,
                );
              }
            }
          }
        }
      }
      _ => {}
    }
  }

  fn check_pat_and_expr(&mut self, left: &Pat, right: &Expr) {
    match (left, right) {
      (Pat::Expr(l_expr), _) => {
        self.check_expr_and_expr(&**l_expr, right);
      }
      (Pat::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(&l_ident.id, r_ident);
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
              left_elem.as_ref().unwrap(),
              &*right_elem.as_ref().unwrap(),
            );
          }

          if let Some(elem) = right_elem {
            if elem.spread.is_some() {
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
              )
            }
          }
        }
      }
      _ => {}
    }
  }
}

impl<'c, 'view> VisitAll for NoSelfAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr) {
    if assign_expr.op == AssignOp::Assign {
      match &assign_expr.left {
        PatOrExpr::Pat(l_pat) => {
          self.check_pat_and_expr(l_pat, &assign_expr.right);
        }
        PatOrExpr::Expr(l_expr) => {
          self.check_expr_and_expr(l_expr, &assign_expr.right);
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
          col: 14,
          message: variant!(NoSelfAssignMessage, Invalid, "a"),
          hint: NoSelfAssignHint::Mistake,
        },
        {
          col: 11,
          message: variant!(NoSelfAssignMessage, Invalid, "b"),
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
    };
  }
}
