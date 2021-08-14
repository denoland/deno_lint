// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_ecmascript::ast::{Expr, ThrowStmt};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoThrowLiteral;

const CODE: &str = "no-throw-literal";

#[derive(Display)]
enum NoThrowLiteralMessage {
  #[display(fmt = "expected an error object to be thrown")]
  ErrObjectExpected,

  #[display(fmt = "do not throw undefined")]
  Undefined,
}

impl LintRule for NoThrowLiteral {
  fn new() -> Box<Self> {
    Box::new(NoThrowLiteral)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoThrowLiteralVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_throw_literal.md")
  }
}

struct NoThrowLiteralVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoThrowLiteralVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoThrowLiteralVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_throw_stmt(&mut self, throw_stmt: &ThrowStmt, _parent: &dyn Node) {
    match &*throw_stmt.arg {
      Expr::Lit(_) => self.context.add_diagnostic(
        throw_stmt.span,
        CODE,
        NoThrowLiteralMessage::ErrObjectExpected,
      ),
      Expr::Ident(ident) if ident.sym == *"undefined" => {
        self.context.add_diagnostic(
          throw_stmt.span,
          CODE,
          NoThrowLiteralMessage::Undefined,
        )
      }
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_throw_literal_valid() {
    assert_lint_ok! {
      NoThrowLiteral,
      "throw e",
    };
  }

  #[test]
  fn no_throw_literal_invalid() {
    assert_lint_err! {
      NoThrowLiteral,
      r#"throw 'kumiko'"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw true"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw 1096"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw null"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::ErrObjectExpected,
      }],
      r#"throw undefined"#: [
      {
        col: 0,
        message: NoThrowLiteralMessage::Undefined,
      }],
    }
  }
}
