// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{
  CallExpr, Callee, CondExpr, DoWhileStmt, Expr, ExprOrSpread, ForStmt, Ident,
  IfStmt, NewExpr, ParenExpr, UnaryExpr, UnaryOp, WhileStmt,
};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
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
  fn new() -> Arc<Self> {
    Arc::new(NoExtraBooleanCast)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoExtraBooleanCastHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_extra_boolean_cast.md")
  }
}

struct NoExtraBooleanCastHandler;

fn unexpected_call(span: SourceRange, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoExtraBooleanCastMessage::BooleanCall,
    NoExtraBooleanCastHint::BooleanCall,
  );
}

fn unexpected_negation(span: SourceRange, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoExtraBooleanCastMessage::DoubleNegation,
    NoExtraBooleanCastHint::DoubleNegation,
  );
}

fn check_condition(expr: &Expr, ctx: &mut Context) {
  match expr {
    Expr::Call(CallExpr {
      ref callee, inner, ..
    }) => {
      if callee_is_boolean(callee) {
        unexpected_call(inner.span, ctx);
      }
    }
    Expr::Unary(UnaryExpr { inner, ref arg, .. }) if has_n_bang(arg, 1) => {
      unexpected_negation(inner.span, ctx);
    }
    Expr::Paren(ParenExpr { ref expr, .. }) => {
      check_condition(expr, ctx);
    }
    _ => (),
  }
}

fn check_unary_expr(unary_expr: &UnaryExpr, ctx: &mut Context) {
  if unary_expr.op() == UnaryOp::Bang {
    let expr = &unary_expr.arg;
    check_unary_expr_internal(unary_expr.range(), expr, ctx);
  }
}

fn check_unary_expr_internal(
  unary_expr_span: SourceRange,
  internal_expr: &Expr,
  ctx: &mut Context,
) {
  match internal_expr {
    Expr::Call(CallExpr { ref callee, .. }) => {
      if callee_is_boolean(callee) {
        unexpected_call(unary_expr_span, ctx);
      }
    }
    Expr::Unary(UnaryExpr { ref arg, .. }) if has_n_bang(arg, 1) => {
      unexpected_negation(unary_expr_span, ctx);
    }
    Expr::Paren(ParenExpr { ref expr, .. }) => {
      check_unary_expr_internal(unary_expr_span, expr, ctx);
    }
    _ => (),
  }
}

impl Handler for NoExtraBooleanCastHandler {
  fn cond_expr(&mut self, cond_expr: &CondExpr, ctx: &mut Context) {
    check_condition(&cond_expr.test, ctx);
  }

  fn for_stmt(&mut self, for_stmt: &ForStmt, ctx: &mut Context) {
    if let Some(ref test_expr) = for_stmt.test {
      check_condition(test_expr, ctx);
    }
  }

  fn if_stmt(&mut self, if_stmt: &IfStmt, ctx: &mut Context) {
    check_condition(&if_stmt.test, ctx);
  }

  fn while_stmt(&mut self, while_stmt: &WhileStmt, ctx: &mut Context) {
    check_condition(&while_stmt.test, ctx);
  }

  fn do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt, ctx: &mut Context) {
    check_condition(&do_while_stmt.test, ctx);
  }

  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if callee_is_boolean(&call_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) = call_expr.args.get(0) {
        check_condition(&*expr, ctx);
      }
    }
  }

  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    if expr_callee_is_boolean(&new_expr.callee) {
      if let Some(ExprOrSpread { expr, .. }) =
        new_expr.args.as_ref().and_then(|a| a.get(0))
      {
        check_condition(&*expr, ctx);
      }
    }
  }

  fn unary_expr(&mut self, unary_expr: &UnaryExpr, ctx: &mut Context) {
    check_unary_expr(unary_expr, ctx);
  }
}

fn callee_is_boolean(callee: &Callee) -> bool {
  match callee {
    Callee::Expr(ref callee) => expr_callee_is_boolean(callee),
    _ => false,
  }
}

fn expr_callee_is_boolean(expr: &Expr) -> bool {
  matches!(expr, Expr::Ident(Ident { inner, .. }) if inner.sym == *"Boolean")
}

/// Checks if `expr` has `n` continuous bang operators at the beginning, ignoring parentheses.
fn has_n_bang(expr: &Expr, n: usize) -> bool {
  if n == 0 {
    return true;
  }

  match expr {
    Expr::Unary(UnaryExpr { ref arg, inner, .. }) => {
      if inner.op == UnaryOp::Bang {
        has_n_bang(arg, n - 1)
      } else {
        false
      }
    }
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
