// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view::TsNonNullExpr;
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoNonNullAssertion;

const CODE: &str = "no-non-null-assertion";

#[derive(Display)]
enum NoNonNullAssertionMessage {
  #[display(fmt = "do not use non-null assertion")]
  Unexpected,
}

impl LintRule for NoNonNullAssertion {
  fn new() -> Arc<Self> {
    Arc::new(NoNonNullAssertion)
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
    NoNonNullAssertionHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_non_null_assertion.md")
  }
}

struct NoNonNullAssertionHandler;

impl Handler for NoNonNullAssertionHandler {
  fn ts_non_null_expr(
    &mut self,
    non_null_expr: &TsNonNullExpr,
    ctx: &mut Context,
  ) {
    if !non_null_expr.parent().is::<TsNonNullExpr>() {
      ctx.add_diagnostic(
        non_null_expr.span(),
        CODE,
        NoNonNullAssertionMessage::Unexpected,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_non_null_assertion_valid() {
    assert_lint_ok! {
      NoNonNullAssertion,
      "instance.doWork();",
      "foo.bar?.includes('baz')",
      "x;",
      "x.y;",
      "x.y.z;",
      "x?.y.z;",
      "x?.y?.z;",
      "!x;",
    };
  }

  #[test]
  fn no_non_null_assertion_invalid() {
    assert_lint_err! {
      NoNonNullAssertion,

      r#"instance!.doWork()"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"foo.bar!.includes('baz');"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y.z!?.();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!?.y.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!?.[y].z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      // r#"x.y.z!!();"#: [
      // {
      //   col: 0,
      //   message: NoNonNullAssertionMessage::Unexpected,
      // }],
      // r#"x.y!!;"#: [
      // {
      //   col: 0,
      //   message: NoNonNullAssertionMessage::Unexpected,
      // }],
      // r#"x!!.y;"#: [
      // {
      //   col: 0,
      //   message: NoNonNullAssertionMessage::Unexpected,
      // }],
      // r#"x!!!;"#: [
      // {
      //   col: 0,
      //   message: NoNonNullAssertionMessage::Unexpected,
      // }],
      r#"x.y?.z!();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y.z!();"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x![y]?.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x![y];"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"!x!.y;"#: [
      {
        col: 1,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!.y?.z;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x.y!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!.y;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],
      r#"x!;"#: [
      {
        col: 0,
        message: NoNonNullAssertionMessage::Unexpected,
      }],

    }
  }
}
