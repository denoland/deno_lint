// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;

use swc_common::Span;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::AssignOp;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSpread;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::ObjectPatProp;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::PatOrExpr;
use swc_ecmascript::ast::Prop;
use swc_ecmascript::ast::PropOrSpread;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoSelfAssign;

impl LintRule for NoSelfAssign {
  fn new() -> Box<Self> {
    Box::new(NoSelfAssign)
  }

  fn code(&self) -> &'static str {
    "no-self-assign"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoSelfAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoSelfAssignVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoSelfAssignVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, name: &str) {
    self.context.add_diagnostic(
      span,
      "no-self-assign",
      &format!("\"{}\" is assigned to itself", name),
    );
  }

  fn is_same_property(
    &mut self,
    left: &MemberExpr,
    right: &MemberExpr,
  ) -> bool {
    if left.computed == right.computed {
      match (&*left.prop, &*right.prop) {
        (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
          if self.is_same_ident(l_ident, r_ident) {
            return true;
          }
        }
        (Expr::Lit(l_lit), Expr::Lit(r_lit)) => {
          if l_lit.get_key() == r_lit.get_key() {
            return true;
          }
        }
        _ => {}
      };
    }

    let left_name = if left.computed { None } else { left.get_key() };
    let right_name = if right.computed {
      None
    } else {
      right.get_key()
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

    if left.obj.is_super_() || right.obj.is_super_() {
      return false;
    }

    match (&left.obj, &right.obj) {
      (ExprOrSuper::Expr(l_boxed_expr), ExprOrSuper::Expr(r_boxed_expr)) => {
        match (&**l_boxed_expr, &**r_boxed_expr) {
          (Expr::Member(l_member_expr), Expr::Member(r_member_expr)) => {
            self.is_same_member(&l_member_expr, &r_member_expr)
          }
          (Expr::This(_), Expr::This(_)) => true,
          (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
            self.is_same_ident(&l_ident, &r_ident)
          }
          _ => false,
        }
      }
      _ => false,
    }
  }

  fn check_same_member(&mut self, left: &MemberExpr, right: &MemberExpr) {
    if self.is_same_member(left, right) {
      let name = (&*right.prop).get_key().expect("Should be identifier");
      self.add_diagnostic(right.span, &name);
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
          let left_name = (&key_val_pat_prop.key).get_key();
          let right_name = (&key_value_prop.key).get_key();

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
        self.check_same_ident(l_ident, r_ident);
      }
      (Pat::Array(l_array_pat), Expr::Array(r_array_lit)) => {
        let end =
          std::cmp::min(l_array_pat.elems.len(), r_array_lit.elems.len());
        for i in 0..end {
          let left_elem = &l_array_pat.elems[i];
          let right_elem = &r_array_lit.elems[i];
          // Avoid cases such as [...a] = [...a, 1]
          if let Some(elem) = left_elem {
            if let Pat::Rest(_) = elem {
              if i < r_array_lit.elems.len() - 1 {
                break;
              }
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

impl<'c> Visit for NoSelfAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_assign_expr(
    &mut self,
    assign_expr: &AssignExpr,
    _parent: &dyn Node,
  ) {
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
  use crate::test_util::*;

  #[test]
  fn no_self_assign_valid() {
    assert_lint_ok_n::<NoSelfAssign>(vec![
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
    ]);
  }

  #[test]
  fn no_self_assign_invalid() {
    assert_lint_err::<NoSelfAssign>("a = a", 4);
    assert_lint_err::<NoSelfAssign>("[a] = [a]", 7);
    assert_lint_err_n::<NoSelfAssign>("[a, b] = [a, b]", vec![10, 13]);
    assert_lint_err::<NoSelfAssign>("[a, b] = [a, c]", 10);
    assert_lint_err::<NoSelfAssign>("[a, b] = [, b]", 12);
    assert_lint_err_n::<NoSelfAssign>("[a, ...b] = [a, ...b]", vec![13, 19]);
    assert_lint_err_n::<NoSelfAssign>("[[a], {b}] = [[a], {b}]", vec![15, 20]);
    assert_lint_err::<NoSelfAssign>("({a} = {a})", 8);
    assert_lint_err::<NoSelfAssign>("({a: b} = {a: b})", 14);
    assert_lint_err::<NoSelfAssign>("({'a': b} = {'a': b})", 18);
    assert_lint_err::<NoSelfAssign>("({a: b} = {'a': b})", 16);
    assert_lint_err::<NoSelfAssign>("({'a': b} = {a: b})", 16);
    assert_lint_err::<NoSelfAssign>("({1: b} = {1: b})", 14);
    assert_lint_err::<NoSelfAssign>("({1: b} = {'1': b})", 16);
    assert_lint_err::<NoSelfAssign>("({'1': b} = {1: b})", 16);
    assert_lint_err::<NoSelfAssign>("({['a']: b} = {a: b})", 18);
    assert_lint_err::<NoSelfAssign>("({'a': b} = {[`a`]: b})", 20);
    assert_lint_err::<NoSelfAssign>("({1: b} = {[1]: b})", 16);
    assert_lint_err_n::<NoSelfAssign>("({a, b} = {a, b})", vec![11, 14]);
    assert_lint_err_n::<NoSelfAssign>("({a, b} = {b, a})", vec![14, 11]);
    assert_lint_err::<NoSelfAssign>("({a, b} = {c, a})", 14);
    assert_lint_err_n::<NoSelfAssign>(
      "({a: {b}, c: [d]} = {a: {b}, c: [d]})",
      vec![25, 33],
    );
    assert_lint_err::<NoSelfAssign>("({a, b} = {a, ...x, b})", 20);
    assert_lint_err::<NoSelfAssign>("a.b = a.b", 6);
    assert_lint_err::<NoSelfAssign>("a.b.c = a.b.c", 8);
    assert_lint_err::<NoSelfAssign>("a[b] = a[b]", 7);
    assert_lint_err::<NoSelfAssign>("a['b'] = a['b']", 9);
    assert_lint_err_on_line::<NoSelfAssign>(
      "a[\n    'b'] = a[\n    'b']",
      2,
      11,
    );
    assert_lint_err::<NoSelfAssign>("this.x = this.x", 9);
    assert_lint_err::<NoSelfAssign>("a['/(?<zero>0)/'] = a[/(?<zero>0)/]", 20);
  }
}
