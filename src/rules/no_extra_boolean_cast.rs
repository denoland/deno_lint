// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{
  CallExpr, CondExpr, DoWhileStmt, Expr, ExprOrSpread, ExprOrSuper, ForStmt,
  Ident, IfStmt, NewExpr, ParenExpr, UnaryExpr, UnaryOp, WhileStmt,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoExtraBooleanCast;

impl LintRule for NoExtraBooleanCast {
  fn new() -> Box<Self> {
    Box::new(NoExtraBooleanCast)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-extra-boolean-cast"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoExtraBooleanCastVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoExtraBooleanCastVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoExtraBooleanCastVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn unexpected_call(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-extra-boolean-cast",
      "Redundant Boolean call.",
    );
  }

  fn unexpected_negation(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-extra-boolean-cast",
      "Redundant double negation.",
    );
  }

  fn check_condition(&mut self, expr: &Expr) {
    match expr {
      Expr::Call(CallExpr {
        ref callee, span, ..
      }) => {
        if expr_or_super_callee_is_boolean(callee) {
          self.unexpected_call(*span);
        }
      }
      Expr::Unary(UnaryExpr {
        span,
        op: UnaryOp::Bang,
        ref arg,
      }) if has_n_bang(arg, 1) => {
        self.unexpected_negation(*span);
      }
      Expr::Paren(ParenExpr { ref expr, .. }) => {
        self.check_condition(expr);
      }
      _ => (),
    }
  }

  fn check_unary_expr(&mut self, unary_expr: &UnaryExpr) {
    if unary_expr.op == UnaryOp::Bang {
      let expr = &*unary_expr.arg;
      self.check_unary_expr_internal(unary_expr.span, expr);
    }
  }

  fn check_unary_expr_internal(
    &mut self,
    unary_expr_span: Span,
    internal_expr: &Expr,
  ) {
    match internal_expr {
      Expr::Call(CallExpr { ref callee, .. }) => {
        if expr_or_super_callee_is_boolean(callee) {
          self.unexpected_call(unary_expr_span);
        }
      }
      Expr::Unary(UnaryExpr {
        op: UnaryOp::Bang,
        ref arg,
        ..
      }) if has_n_bang(arg, 1) => {
        self.unexpected_negation(unary_expr_span);
      }
      Expr::Paren(ParenExpr { ref expr, .. }) => {
        self.check_unary_expr_internal(unary_expr_span, expr);
      }
      _ => (),
    }
  }
}

impl<'c> Visit for NoExtraBooleanCastVisitor<'c> {
  noop_visit_type!();

  fn visit_cond_expr(&mut self, cond_expr: &CondExpr, parent: &dyn Node) {
    self.check_condition(&*cond_expr.test);
    swc_ecmascript::visit::visit_cond_expr(self, cond_expr, parent);
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, parent: &dyn Node) {
    if let Some(ref test_expr) = for_stmt.test {
      self.check_condition(&**test_expr);
    }
    swc_ecmascript::visit::visit_for_stmt(self, for_stmt, parent);
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, parent: &dyn Node) {
    self.check_condition(&*if_stmt.test);
    swc_ecmascript::visit::visit_if_stmt(self, if_stmt, parent);
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, parent: &dyn Node) {
    self.check_condition(&*while_stmt.test);
    swc_ecmascript::visit::visit_while_stmt(self, while_stmt, parent);
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &DoWhileStmt,
    parent: &dyn Node,
  ) {
    self.check_condition(&*do_while_stmt.test);
    swc_ecmascript::visit::visit_do_while_stmt(self, do_while_stmt, parent);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, parent: &dyn Node) {
    if expr_or_super_callee_is_boolean(&call_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) = call_expr.args.get(0) {
        self.check_condition(&*expr);
      }
    }
    swc_ecmascript::visit::visit_call_expr(self, call_expr, parent);
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, parent: &dyn Node) {
    if expr_callee_is_boolean(&new_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) =
        new_expr.args.as_ref().and_then(|a| a.get(0))
      {
        self.check_condition(&*expr);
      }
    }
    swc_ecmascript::visit::visit_new_expr(self, new_expr, parent);
  }

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, parent: &dyn Node) {
    self.check_unary_expr(unary_expr);
    swc_ecmascript::visit::visit_unary_expr(self, unary_expr, parent);
  }
}

fn expr_or_super_callee_is_boolean(expr_or_super: &ExprOrSuper) -> bool {
  match expr_or_super {
    ExprOrSuper::Expr(ref callee) => expr_callee_is_boolean(&**callee),
    _ => false,
  }
}

fn expr_callee_is_boolean(expr: &Expr) -> bool {
  matches!(expr, Expr::Ident(Ident { ref sym, .. }) if sym == "Boolean")
}

/// Checks if `expr` has `n` continuous bang operators at the beginning, ignoring parentheses.
fn has_n_bang(expr: &Expr, n: usize) -> bool {
  if n == 0 {
    return true;
  }

  match expr {
    Expr::Unary(UnaryExpr {
      op: UnaryOp::Bang,
      ref arg,
      ..
    }) => has_n_bang(arg, n - 1),
    Expr::Paren(ParenExpr { ref expr, .. }) => has_n_bang(expr, n),
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_extra_boolean_cast_valid() {
    assert_lint_ok! {
      NoExtraBooleanCast,
      "Boolean(bar, !!baz);",
      "var foo = !!bar;",
      "function foo() { return !!bar; }",
      "var foo = bar() ? !!baz : !!bat",
      "for(!!foo;;) {}",
      "for(;; !!foo) {}",
      "var foo = Boolean(bar);",
      "function foo() { return Boolean(bar); }",
      "var foo = bar() ? Boolean(baz) : Boolean(bat)",
      "for(Boolean(foo);;) {}",
      "for(;; Boolean(foo)) {}",
      "if (new Boolean(foo)) {}",
      "if (!!foo || bar) {}",
    };
  }

  #[test]
  fn no_extra_boolean_cast_invalid() {
    assert_lint_err::<NoExtraBooleanCast>("if (!!foo) {}", 4);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!foo)", 13);
    assert_lint_err::<NoExtraBooleanCast>("while (!!foo) {}", 7);
    assert_lint_err::<NoExtraBooleanCast>("!!foo ? bar : baz", 0);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!foo;) {}", 7);
    assert_lint_err::<NoExtraBooleanCast>("!!!foo", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!foo)", 8);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!foo)", 12);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean(foo)) {}", 4);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (Boolean(foo))", 13);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean(foo)) {}", 7);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(foo) ? bar : baz", 0);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean(foo);) {}", 7);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo && bar)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo + bar)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(+foo)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo())", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo = bar)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(...foo);", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo, bar());", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean((foo, bar()));", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean();", 0);
    assert_lint_err::<NoExtraBooleanCast>("!(Boolean());", 0);
    assert_lint_err::<NoExtraBooleanCast>("if (!Boolean()) { foo() }", 4);
    assert_lint_err::<NoExtraBooleanCast>("while (!Boolean()) { foo() }", 7);
    assert_lint_err::<NoExtraBooleanCast>(
      "var foo = Boolean() ? bar() : baz()",
      10,
    );
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean()) { foo() }", 4);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean()) { foo() }", 7);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(Boolean(foo))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!foo, bar)", 8);
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield!!a ? b : c }",
      23,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield!! a ? b : c }",
      23,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield! !a ? b : c }",
      23,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield !!a ? b : c }",
      24,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield(!!a) ? b : c }",
      24,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "function *foo() { yield/**/!!a ? b : c }",
      27,
    );
    assert_lint_err::<NoExtraBooleanCast>("x=!!a ? b : c ", 2);
    assert_lint_err::<NoExtraBooleanCast>("void!Boolean()", 4);
    assert_lint_err::<NoExtraBooleanCast>("void! Boolean()", 4);
    assert_lint_err::<NoExtraBooleanCast>("typeof!Boolean()", 6);
    assert_lint_err::<NoExtraBooleanCast>("(!Boolean())", 1);
    assert_lint_err::<NoExtraBooleanCast>("+!Boolean()", 1);
    assert_lint_err::<NoExtraBooleanCast>("void !Boolean()", 5);
    assert_lint_err::<NoExtraBooleanCast>("void(!Boolean())", 5);
    assert_lint_err::<NoExtraBooleanCast>("void/**/!Boolean()", 8);
    assert_lint_err::<NoExtraBooleanCast>("!/**/!!foo", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!/**/!foo", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!/**/foo", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!foo/**/", 0);
    assert_lint_err::<NoExtraBooleanCast>("if(!/**/!foo);", 3);
    assert_lint_err::<NoExtraBooleanCast>("(!!/**/foo ? 1 : 2)", 1);
    assert_lint_err::<NoExtraBooleanCast>("!/**/Boolean(foo)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean/**/(foo)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(/**/foo)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo/**/)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(foo)/**/", 0);
    assert_lint_err::<NoExtraBooleanCast>("if(Boolean/**/(foo));", 3);
    assert_lint_err::<NoExtraBooleanCast>("(Boolean(foo/**/) ? 1 : 2)", 1);
    assert_lint_err::<NoExtraBooleanCast>("/**/!Boolean()", 4);
    assert_lint_err::<NoExtraBooleanCast>("!/**/Boolean()", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean/**/()", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(/**/)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean()/**/", 0);
    assert_lint_err::<NoExtraBooleanCast>("if(!/**/Boolean());", 3);
    assert_lint_err::<NoExtraBooleanCast>("(!Boolean(/**/) ? 1 : 2)", 1);
    assert_lint_err::<NoExtraBooleanCast>("if(/**/Boolean());", 7);
    assert_lint_err::<NoExtraBooleanCast>("if(Boolean/**/());", 3);
    assert_lint_err::<NoExtraBooleanCast>("if(Boolean(/**/));", 3);
    assert_lint_err::<NoExtraBooleanCast>("if(Boolean()/**/);", 3);
    assert_lint_err::<NoExtraBooleanCast>("(Boolean/**/() ? 1 : 2)", 1);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!(a, b))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(Boolean((a, b)))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((!!(a, b)))", 9);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((Boolean((a, b))))", 9);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!(!(a, b)))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((!(!(a, b))))", 9);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!(a = b))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((!!(a = b)))", 9);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(Boolean(a = b))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(Boolean((a += b)))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!(a === b))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!((a !== b)))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!!a.b)", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(Boolean((a)))", 8);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((!!(a)))", 9);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!(a, b))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(Boolean((a, b)))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean((!!(a, b)))", 13);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean((Boolean((a, b))))", 13);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!(!(a, b)))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean((!(!(a, b))))", 13);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!(a = b))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean((!!(a = b)))", 13);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(Boolean(a = b))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(Boolean((a += b)))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!(a === b))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!((a !== b)))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(!!a.b)", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean(Boolean((a)))", 12);
    assert_lint_err::<NoExtraBooleanCast>("new Boolean((!!(a)))", 13);
    assert_lint_err::<NoExtraBooleanCast>("if (!!(a, b));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean((a, b)));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (!(!(a, b)));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (!!(a = b));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean(a = b));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (!!(a > b));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean(a === b));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (!!f(a));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean(f(a)));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (!!(f(a)));", 4);
    assert_lint_err::<NoExtraBooleanCast>("if ((!!f(a)));", 5);
    assert_lint_err::<NoExtraBooleanCast>("if ((Boolean(f(a))));", 5);
    assert_lint_err::<NoExtraBooleanCast>("if (!!a);", 4);
    assert_lint_err::<NoExtraBooleanCast>("if (Boolean(a));", 4);
    assert_lint_err::<NoExtraBooleanCast>("while (!!(a, b));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean((a, b)));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (!(!(a, b)));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (!!(a = b));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean(a = b));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (!!(a > b));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean(a === b));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (!!f(a));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean(f(a)));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (!!(f(a)));", 7);
    assert_lint_err::<NoExtraBooleanCast>("while ((!!f(a)));", 8);
    assert_lint_err::<NoExtraBooleanCast>("while ((Boolean(f(a))));", 8);
    assert_lint_err::<NoExtraBooleanCast>("while (!!a);", 7);
    assert_lint_err::<NoExtraBooleanCast>("while (Boolean(a));", 7);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!(a, b));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (Boolean((a, b)));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!(!(a, b)));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!(a = b));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (Boolean(a = b));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!(a > b));", 13);
    assert_lint_err::<NoExtraBooleanCast>(
      "do {} while (Boolean(a === b));",
      13,
    );
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!f(a));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (Boolean(f(a)));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!(f(a)));", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while ((!!f(a)));", 14);
    assert_lint_err::<NoExtraBooleanCast>("do {} while ((Boolean(f(a))));", 14);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (!!a);", 13);
    assert_lint_err::<NoExtraBooleanCast>("do {} while (Boolean(a));", 13);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!(a, b););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean((a, b)););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; !(!(a, b)););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!(a = b););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean(a = b););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!(a > b););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean(a === b););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!f(a););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean(f(a)););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!(f(a)););", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; (!!f(a)););", 8);
    assert_lint_err::<NoExtraBooleanCast>("for (; (Boolean(f(a))););", 8);
    assert_lint_err::<NoExtraBooleanCast>("for (; !!a;);", 7);
    assert_lint_err::<NoExtraBooleanCast>("for (; Boolean(a););", 7);
    assert_lint_err::<NoExtraBooleanCast>("!!(a, b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("(!!(a, b)) ? c : d", 1);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((a, b)) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!(a = b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a -= b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("(Boolean((a *= b))) ? c : d", 1);
    assert_lint_err::<NoExtraBooleanCast>("!!(a ? b : c) ? d : e", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a ? b : c) ? d : e", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!(a || b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a && b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!(a === b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a < b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!((a !== b)) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean((a >= b)) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!+a ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!+(a) ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(!a) ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!f(a) ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("(!!f(a)) ? b : c", 1);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a.b) ? c : d", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!a ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("Boolean(a) ? b : c", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a, b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean((a, b))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a = b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!(!(a += b))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!(!!(a += b))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a -= b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean((a -= b))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!(Boolean(a -= b))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a || b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a || b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a && b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a && b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a != b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a === b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("var x = !Boolean(a > b)", 8);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a - b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(a ** b)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a ** b)", 0);
    assert_lint_err::<NoExtraBooleanCast>(
      "async function f() { !!!(await a) }",
      21,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "async function f() { !Boolean(await a) }",
      21,
    );
    assert_lint_err_n::<NoExtraBooleanCast>("!!!!a", vec![0, 1]);
    assert_lint_err_n::<NoExtraBooleanCast>("!!(!(!a))", vec![0, 1]);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(!a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean((!a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(!(a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!(Boolean(!a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!+a", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(+a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!(!+a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!(!!+a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean((-a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(-(a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(--a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a++)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!f(a)", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!(f(a))", 0);
    assert_lint_err::<NoExtraBooleanCast>("!!!a", 0);
    assert_lint_err::<NoExtraBooleanCast>("!Boolean(a)", 0);

    assert_lint_err_n::<NoExtraBooleanCast>("!Boolean(!!a)", vec![0, 9]);
    assert_lint_err_n::<NoExtraBooleanCast>("!Boolean(Boolean(a))", vec![0, 9]);
    assert_lint_err_n::<NoExtraBooleanCast>(
      "!Boolean(Boolean(!!a))",
      vec![0, 9, 17],
    );
    assert_lint_err::<NoExtraBooleanCast>("while (a) { if (!!b) {} }", 16);
    assert_lint_err::<NoExtraBooleanCast>(
      "while (a) { if (Boolean(b)) {} }",
      16,
    );
    assert_lint_err::<NoExtraBooleanCast>("if (a) { const b = !!!c; }", 19);
    assert_lint_err::<NoExtraBooleanCast>(
      "if (a) { const b = !Boolean(c); }",
      19,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "for (let a = 0; a < n; a++) { if (!!b) {} }",
      34,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "for (let a = 0; a < n; a++) { if (Boolean(b)) {} }",
      34,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "do { const b = !!!c; } while(a)",
      15,
    );
    assert_lint_err::<NoExtraBooleanCast>(
      "do { const b = !Boolean(c); } while(a)",
      15,
    );
    assert_lint_err::<NoExtraBooleanCast>("a ? !!!b : c", 4);
    assert_lint_err::<NoExtraBooleanCast>("a ? b : !!!c", 8);
    assert_lint_err_n::<NoExtraBooleanCast>("a ? !!!b : !!!c", vec![4, 11]);
    assert_lint_err::<NoExtraBooleanCast>("a ? !Boolean(b) : c", 4);
    assert_lint_err::<NoExtraBooleanCast>("a ? b : !Boolean(c)", 8);
    assert_lint_err_n::<NoExtraBooleanCast>(
      "a ? !Boolean(b) : !Boolean(c)",
      vec![4, 18],
    );
  }
}
