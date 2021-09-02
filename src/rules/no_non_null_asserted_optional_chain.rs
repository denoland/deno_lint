// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{Expr, ExprOrSuper};
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;
use derive_more::Display;

use deno_ast::swc::common::Span;

pub struct NoNonNullAssertedOptionalChain;

const CODE: &str = "no-non-null-asserted-optional-chain";

#[derive(Display)]
enum NoNonNullAssertedOptionalChainMessage {
  #[display(
    fmt = "Optional chain expressions can return undefined by design - using a non-null assertion is unsafe and wrong."
  )]
  WrongAssertion,
}

impl LintRule for NoNonNullAssertedOptionalChain {
  fn new() -> Box<Self> {
    Box::new(NoNonNullAssertedOptionalChain)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoNonNullAssertedOptionalChainVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_non_null_asserted_optional_chain.md")
  }
}

struct NoNonNullAssertedOptionalChainVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoNonNullAssertedOptionalChainVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic(
      span,
      CODE,
      NoNonNullAssertedOptionalChainMessage::WrongAssertion,
    );
  }

  fn check_expr_for_nested_optional_assert(&mut self, span: Span, expr: &Expr) {
    if let Expr::OptChain(_) = expr {
      self.add_diagnostic(span)
    }
  }
}

impl<'c, 'view> Visit for NoNonNullAssertedOptionalChainVisitor<'c, 'view> {
  fn visit_ts_non_null_expr(
    &mut self,
    ts_non_null_expr: &deno_ast::swc::ast::TsNonNullExpr,
    _parent: &dyn Node,
  ) {
    match &*ts_non_null_expr.expr {
      Expr::Member(member_expr) => {
        if let ExprOrSuper::Expr(expr) = &member_expr.obj {
          self
            .check_expr_for_nested_optional_assert(ts_non_null_expr.span, expr);
        }
      }
      Expr::Call(call_expr) => {
        if let ExprOrSuper::Expr(expr) = &call_expr.callee {
          self
            .check_expr_for_nested_optional_assert(ts_non_null_expr.span, expr);
        }
      }
      Expr::Paren(paren_expr) => self.check_expr_for_nested_optional_assert(
        ts_non_null_expr.span,
        &*paren_expr.expr,
      ),
      _ => {}
    };

    self.check_expr_for_nested_optional_assert(
      ts_non_null_expr.span,
      &*ts_non_null_expr.expr,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_non_null_asserted_optional_chain_valid() {
    assert_lint_ok! {
      NoNonNullAssertedOptionalChain,
      "foo.bar!;",
      "foo.bar()!;",
      "foo?.bar();",
      "foo?.bar;",
      "(foo?.bar).baz!;",
      "(foo?.bar()).baz!;",
    };
  }

  #[test]
  fn no_non_null_asserted_optional_chain_invalid() {
    assert_lint_err! {
      NoNonNullAssertedOptionalChain,
      r#"foo?.bar!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo?.['bar']!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo?.bar()!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"foo.bar?.()!;"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!.baz"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!().baz"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar)!()"#: [
      {
        col: 0,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar!)"#: [
      {
        col: 1,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
      r#"(foo?.bar!)()"#: [
      {
        col: 1,
        message: NoNonNullAssertedOptionalChainMessage::WrongAssertion,
      }],
    }
  }
}
