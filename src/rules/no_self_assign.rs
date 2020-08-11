// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::AssignOp;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::ExprOrSpread;
use swc_ecmascript::ast::PatOrExpr;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_common::Span;
use std::sync::Arc;
use crate::swc_util::Key;

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
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoSelfAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoSelfAssignVisitor {
  context: Arc<Context>,
}

impl NoSelfAssignVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, name: &str) {
    self
      .context
      .add_diagnostic(span, "no-self-assign", &format!("\"{}\" is assigned to itself", name));
  }

  fn is_same_property(&mut self, left: &MemberExpr, right: &MemberExpr) -> bool {
    if left.computed == right.computed {
      match (&*left.prop, &*right.prop) {
        (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
          if self.is_same_ident(l_ident, r_ident) {
            return true;
          }
        },
        _ => {}
      };
    }

    let left_name = (&*left.prop).get_key();
    let right_name = (&*right.prop).get_key();

    if let Some(lname) = left_name {
      if let Some(rname) = right_name {
        return lname == rname;
      }
    }

    false
  }

  fn is_same_member(&mut self, left: &MemberExpr, right: &MemberExpr) -> bool {
    if !self.is_same_property(left, right) {
      return false;
    }

    if left.obj.is_super_() || right.obj.is_super_() {
      return false;
    }

    match (&left.obj, &right.obj) {
      (ExprOrSuper::Expr(l_boxed_expr), ExprOrSuper::Expr(r_boxed_expr)) => {
        match (&**l_boxed_expr, &**r_boxed_expr) {
          (Expr::Member(l_member_expr), Expr::Member(r_member_expr)) => {
            return self.is_same_member(&l_member_expr, &r_member_expr);
          },
          (Expr::This(_), _) => {
            return true;
          },
          (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
            return self.is_same_ident(&l_ident, &r_ident)
          },
          _ => return false
        }
      },
      _ => return false,
    };
  }

  fn is_same_ident(&mut self, left: &Ident, right: &Ident) -> bool {
    left.sym == right.sym
  }

  fn check_same_ident(&mut self, left: &Ident, right: &Ident) {
    if self.is_same_ident(left, right) {
      self.add_diagnostic(right.span, &right.sym);
    }
  }

  fn each_self_assignment_for_expr(&mut self, left: &Expr, right: &Expr) {
    match (left, right) {
      (Expr::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(l_ident, r_ident);
      },
      (Expr::Member(l_member), Expr::Member(r_member)) => {
        self.is_same_member(l_member, r_member);
      },
      _ => {}
    }
  }

  fn each_self_assignment_for_pat_and_spread_or_expr(&mut self, left: &Pat, right: &ExprOrSpread) {
    if right.spread.is_some() {
      if let Pat::Rest(rest_pat) = left {
        self.each_self_assignment_for_pat(&*rest_pat.arg, &*right.expr)
      }
    } else {
      self.each_self_assignment_for_pat(left, &*right.expr);
    }
  }

  fn each_self_assignment_for_pat(&mut self, left: &Pat, right: &Expr) {
    match (left, right) {
      (Pat::Ident(l_ident), Expr::Ident(r_ident)) => {
        self.check_same_ident(l_ident, r_ident);
      },
      (Pat::Array(l_array_pat), Expr::Array(r_array_lit)) => {
        let end = std::cmp::min(l_array_pat.elems.len(), r_array_lit.elems.len());

        for i in 0..end {
          let left_elem = &l_array_pat.elems[i];
          let right_elem = &r_array_lit.elems[i];

          // Avoid cases such as [...a] = [...a, 1]
          if let Some(elem) = left_elem {
            if let Pat::Rest(_) = elem {
              if i < r_array_lit.elems.len() {
                break;
              }
            }
          }

          if left_elem.is_some() && right_elem.is_some() {
            self.each_self_assignment_for_pat_and_spread_or_expr(left_elem.as_ref().unwrap(), &*right_elem.as_ref().unwrap());
          }

          if let Some(elem) = right_elem {
            if elem.spread.is_some() {
              break;
            }
          }
        }
      },
      (Pat::Object(_), Expr::Object(r_obj)) => {
        if r_obj.props.len() >= 1 {
          todo!();
        }
      },
      _ => {}
    }
  }
}

impl Visit for NoSelfAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _parent: &dyn Node) {
    if assign_expr.op == AssignOp::Assign {
      match &assign_expr.left {
        PatOrExpr::Pat(l_pat) => {
          self.each_self_assignment_for_pat(l_pat, &assign_expr.right);
        },
        PatOrExpr::Expr(l_expr) => {
          self.each_self_assignment_for_expr(l_expr, &assign_expr.right);
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
      "this.x = this.y"
    ]);
  }

  #[test]
  fn no_self_assign_invalid() {
    assert_lint_err::<NoSelfAssign>("a = a", 4);
//     assert_lint_err::<NoSelfAssign>("let a: Object;", 7);
//     assert_lint_err::<NoSelfAssign>("let a: Number;", 7);
//     assert_lint_err::<NoSelfAssign>("let a: Function;", 7);
//     assert_lint_err::<NoSelfAssign>("let a: object;", 7);
//     assert_lint_err::<NoSelfAssign>("let a: {};", 7);
//     assert_lint_err::<NoSelfAssign>("let a: { b: String};", 12);
//     assert_lint_err::<NoSelfAssign>("let a: { b: Number};", 12);
//     assert_lint_err_n::<NoSelfAssign>(
//       "let a: { b: object, c: Object};",
//       vec![12, 23],
//     );
//     assert_lint_err::<NoSelfAssign>("let a: { b: { c : Function}};", 18);
//     assert_lint_err::<NoSelfAssign>("let a: Array<String>", 13);
//     assert_lint_err_n::<NoSelfAssign>("let a: Number<Function>", vec![7, 14]);
//     assert_lint_err::<NoSelfAssign>("function foo(a: String) {}", 16);
//     assert_lint_err::<NoSelfAssign>("function foo(): Number {}", 16);
//     assert_lint_err::<NoSelfAssign>("let a: () => Number;", 13);
//     assert_lint_err::<NoSelfAssign>("'a' as String;", 7);
//     assert_lint_err::<NoSelfAssign>("1 as Number;", 5);
//     assert_lint_err_on_line_n::<NoSelfAssign>(
//       "
// class Foo<F = String> extends Bar<String> implements Baz<Object> {
//   constructor(foo: String | Object) {}
    
//   exit(): Array<String> {
//     const foo: String = 1 as String;
//   }
// }",
//       vec![
//         (2, 14),
//         (2, 34),
//         (2, 57),
//         (3, 19),
//         (3, 28),
//         (5, 16),
//         (6, 15),
//         (6, 29),
//       ],
//     )
  }
}
