// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_common::Span;
use swc_ecmascript::ast::{
  CallExpr, CondExpr, DoWhileStmt, Expr, ExprOrSpread, ExprOrSuper, ForStmt,
  Ident, IfStmt, NewExpr, ParenExpr, UnaryExpr, UnaryOp, WhileStmt,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoExtraBooleanCast;

const CODE: &str = "no-extra-boolean-cast";

#[derive(Display)]
enum NoExtraBooleanCastMessage {
  #[display(fmt = "Redundant Boolean call.")]
  BooleanCall,
  #[display(fmt = "Redundant double negation.")]
  DoubleNegation,
}

#[derive(Display)]
enum NoExtraBooleanCastHint {
  #[display(fmt = "Remove the Boolean call, it is unnecessary")]
  BooleanCall,
  #[display(fmt = "Remove the double negation (`!!`), it is unnecessary")]
  DoubleNegation,
}

impl LintRule for NoExtraBooleanCast {
  fn new() -> Box<Self> {
    Box::new(NoExtraBooleanCast)
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
    let mut visitor = NoExtraBooleanCastVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_extra_boolean_cast.md")
  }
}

struct NoExtraBooleanCastVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoExtraBooleanCastVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn unexpected_call(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoExtraBooleanCastMessage::BooleanCall,
      NoExtraBooleanCastHint::BooleanCall,
    );
  }

  fn unexpected_negation(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoExtraBooleanCastMessage::DoubleNegation,
      NoExtraBooleanCastHint::DoubleNegation,
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

impl<'c, 'view> VisitAll for NoExtraBooleanCastVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_cond_expr(&mut self, cond_expr: &CondExpr, _: &dyn Node) {
    self.check_condition(&*cond_expr.test);
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _: &dyn Node) {
    if let Some(ref test_expr) = for_stmt.test {
      self.check_condition(&**test_expr);
    }
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _: &dyn Node) {
    self.check_condition(&*if_stmt.test);
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, _: &dyn Node) {
    self.check_condition(&*while_stmt.test);
  }

  fn visit_do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt, _: &dyn Node) {
    self.check_condition(&*do_while_stmt.test);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _: &dyn Node) {
    if expr_or_super_callee_is_boolean(&call_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) = call_expr.args.get(0) {
        self.check_condition(&*expr);
      }
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _: &dyn Node) {
    if expr_callee_is_boolean(&new_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) =
        new_expr.args.as_ref().and_then(|a| a.get(0))
      {
        self.check_condition(&*expr);
      }
    }
  }

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _: &dyn Node) {
    self.check_unary_expr(unary_expr);
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
    assert_lint_err! {
      NoExtraBooleanCast,
      "if (!!foo) {}": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (!!foo)": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (!!foo) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!foo ? bar : baz": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; !!foo;) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!foo": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!!foo)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!foo)": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(foo)) {}": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (Boolean(foo))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (Boolean(foo)) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(foo) ? bar : baz": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; Boolean(foo);) {}": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo && bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo + bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(+foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo())": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo = bar)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(...foo);": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo, bar());": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean((foo, bar()));": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean();": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!(Boolean());": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!Boolean()) { foo() }": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!Boolean()) { foo() }": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "var foo = Boolean() ? bar() : baz()": [
        {
          col: 10,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (Boolean()) { foo() }": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (Boolean()) { foo() }": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(Boolean(foo))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(!!foo, bar)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield!!a ? b : c }": [
        {
          col: 23,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield!! a ? b : c }": [
        {
          col: 23,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield! !a ? b : c }": [
        {
          col: 23,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield !!a ? b : c }": [
        {
          col: 24,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield(!!a) ? b : c }": [
        {
          col: 24,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "function *foo() { yield/**/!!a ? b : c }": [
        {
          col: 27,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "x=!!a ? b : c ": [
        {
          col: 2,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "void!Boolean()": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "void! Boolean()": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "typeof!Boolean()": [
        {
          col: 6,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "(!Boolean())": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "+!Boolean()": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "void !Boolean()": [
        {
          col: 5,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "void(!Boolean())": [
        {
          col: 5,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "void/**/!Boolean()": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!/**/!!foo": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!/**/!foo": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!/**/foo": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!foo/**/": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if(!/**/!foo);": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "(!!/**/foo ? 1 : 2)": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!/**/Boolean(foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean/**/(foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(/**/foo)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo/**/)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(foo)/**/": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(Boolean/**/(foo));": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "(Boolean(foo/**/) ? 1 : 2)": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "/**/!Boolean()": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!/**/Boolean()": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean/**/()": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(/**/)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean()/**/": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(!/**/Boolean());": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "(!Boolean(/**/) ? 1 : 2)": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(/**/Boolean());": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(Boolean/**/());": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(Boolean(/**/));": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if(Boolean()/**/);": [
        {
          col: 3,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "(Boolean/**/() ? 1 : 2)": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(!!(a, b))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(Boolean((a, b)))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean((!!(a, b)))": [
        {
          col: 9,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean((Boolean((a, b))))": [
        {
          col: 9,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(!(!(a, b)))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean((!(!(a, b))))": [
        {
          col: 9,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!!(a = b))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean((!!(a = b)))": [
        {
          col: 9,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(Boolean(a = b))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(Boolean((a += b)))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean(!!(a === b))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!!((a !== b)))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!!a.b)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(Boolean((a)))": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "Boolean((!!(a)))": [
        {
          col: 9,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!(a, b))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(Boolean((a, b)))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "new Boolean((!!(a, b)))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean((Boolean((a, b))))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "new Boolean(!(!(a, b)))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean((!(!(a, b))))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!(a = b))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean((!!(a = b)))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(Boolean(a = b))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "new Boolean(Boolean((a += b)))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "new Boolean(!!(a === b))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!((a !== b)))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(!!a.b)": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "new Boolean(Boolean((a)))": [
        {
          col: 12,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "new Boolean((!!(a)))": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (!!(a, b));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean((a, b)));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!(!(a, b)));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (!!(a = b));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(a = b));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!!(a > b));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(a === b));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!!f(a));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(f(a)));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!!(f(a)));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if ((!!f(a)));": [
        {
          col: 5,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if ((Boolean(f(a))));": [
        {
          col: 5,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (!!a);": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (Boolean(a));": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!!(a, b));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (Boolean((a, b)));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!(!(a, b)));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (!!(a = b));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (Boolean(a = b));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!!(a > b));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (Boolean(a === b));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!!f(a));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (Boolean(f(a)));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!!(f(a)));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while ((!!f(a)));": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while ((Boolean(f(a))));": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "while (!!a);": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (Boolean(a));": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!!(a, b));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (Boolean((a, b)));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!(!(a, b)));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (!!(a = b));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (Boolean(a = b));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!!(a > b));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (Boolean(a === b));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!!f(a));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (Boolean(f(a)));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!!(f(a)));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while ((!!f(a)));": [
        {
          col: 14,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while ((Boolean(f(a))));": [
        {
          col: 14,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do {} while (!!a);": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do {} while (Boolean(a));": [
        {
          col: 13,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !!(a, b););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; Boolean((a, b)););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !(!(a, b)););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; !!(a = b););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; Boolean(a = b););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !!(a > b););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; Boolean(a === b););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !!f(a););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; Boolean(f(a)););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !!(f(a)););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; (!!f(a)););": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; (Boolean(f(a))););": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (; !!a;);": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (; Boolean(a););": [
        {
          col: 7,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!(a, b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "(!!(a, b)) ? c : d": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean((a, b)) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!(a = b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a -= b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "(Boolean((a *= b))) ? c : d": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!(a ? b : c) ? d : e": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a ? b : c) ? d : e": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!(a || b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a && b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!(a === b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a < b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!((a !== b)) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean((a >= b)) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!+a ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!+(a) ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(!a) ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!f(a) ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "(!!f(a)) ? b : c": [
        {
          col: 1,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a.b) ? c : d": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!a ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "Boolean(a) ? b : c": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a, b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean((a, b))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a = b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!(!(a += b))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!(!!(a += b))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a -= b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean((a -= b))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!(Boolean(a -= b))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a || b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a || b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a && b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a && b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a != b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!(a === b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "var x = !Boolean(a > b)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(a - b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!(a ** b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a ** b)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "async function f() { !!!(await a) }": [
        {
          col: 21,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "async function f() { !Boolean(await a) }": [
        {
          col: 21,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!!a": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        },
        {
          col: 1,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!(!(!a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        },
        {
          col: 1,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(!a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean((!a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(!(a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!(Boolean(!a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!+a": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!(+a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!(!+a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!(!!+a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean((-a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(-(a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!(--a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a++)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!!!f(a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!(f(a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!!!a": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(!!a)": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 9,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "!Boolean(Boolean(a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 9,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "!Boolean(Boolean(!!a))": [
        {
          col: 0,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 9,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 17,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (a) { if (!!b) {} }": [
        {
          col: 16,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "while (a) { if (Boolean(b)) {} }": [
        {
          col: 16,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "if (a) { const b = !!!c; }": [
        {
          col: 19,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "if (a) { const b = !Boolean(c); }": [
        {
          col: 19,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "for (let a = 0; a < n; a++) { if (!!b) {} }": [
        {
          col: 34,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "for (let a = 0; a < n; a++) { if (Boolean(b)) {} }": [
        {
          col: 34,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "do { const b = !!!c; } while(a)": [
        {
          col: 15,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "do { const b = !Boolean(c); } while(a)": [
        {
          col: 15,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "a ? !!!b : c": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "a ? b : !!!c": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "a ? !!!b : !!!c": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        },
        {
          col: 11,
          message: NoExtraBooleanCastMessage::DoubleNegation,
          hint: NoExtraBooleanCastHint::DoubleNegation,
        }
      ],
      "a ? !Boolean(b) : c": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "a ? b : !Boolean(c)": [
        {
          col: 8,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ],
      "a ? !Boolean(b) : !Boolean(c)": [
        {
          col: 4,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        },
        {
          col: 18,
          message: NoExtraBooleanCastMessage::BooleanCall,
          hint: NoExtraBooleanCastHint::BooleanCall,
        }
      ]
    };
  }
}
