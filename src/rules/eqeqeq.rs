// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_ecmascript::ast::{BinExpr, BinaryOp};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct Eqeqeq;

const CODE: &str = "eqeqeq";

#[derive(Display)]
enum EqeqeqMessage {
  #[display(fmt = "expected '===' and instead saw '=='.")]
  ExpectedEqual,
  #[display(fmt = "expected '!==' and instead saw '!='.")]
  ExpectedNotEqual,
}

#[derive(Display)]
enum EqeqeqHint {
  #[display(fmt = "Use '==='")]
  UseEqeqeq,
  #[display(fmt = "Use '!=='")]
  UseNoteqeq,
}

impl LintRule for Eqeqeq {
  fn new() -> Box<Self> {
    Box::new(Eqeqeq)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = EqeqeqVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/eqeqeq.md")
  }
}

struct EqeqeqVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> EqeqeqVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for EqeqeqVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, parent: &dyn Node) {
    if matches!(bin_expr.op, BinaryOp::EqEq | BinaryOp::NotEq) {
      let (message, hint) = if bin_expr.op == BinaryOp::EqEq {
        (EqeqeqMessage::ExpectedEqual, EqeqeqHint::UseEqeqeq)
      } else {
        (EqeqeqMessage::ExpectedNotEqual, EqeqeqHint::UseNoteqeq)
      };
      self
        .context
        .add_diagnostic_with_hint(bin_expr.span, CODE, message, hint)
    }
    swc_ecmascript::visit::visit_bin_expr(self, bin_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eqeqeq_valid() {
    assert_lint_ok! {
      Eqeqeq,
      "midori === sapphire",
      "midori !== hazuki",
      "kumiko === null",
      "reina !== null",
      "null === null",
      "null !== null",
    };
  }

  #[test]
  fn eqeqeq_invalid() {
    assert_lint_err! {
      Eqeqeq,

      "a == b": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "a != b": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "typeof a == 'number'": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "'string' != typeof a": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "true == true": [
      {
        col: 0,

        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "2 == 3": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "'hello' != 'world'": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "a == null": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "null != a": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "true == null": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "true != null": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "null == null": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "null != null": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      r#"
a
==
b     "#: [
      {
        line: 2,
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a) == b": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a) != b": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "a == (b)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "a != (b)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "(a) == (b)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a) != (b)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "(a == b) == (c)": [
      {
        line: 1,
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      },
      {
        line: 1,
        col: 1,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],

      "(a != b) != (c)": [
      {
        line: 1,
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      },
      {
        line: 1,
        col: 1,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "(a == b) === (c)": [
      {
        col: 1,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a == b) !== (c)": [
      {
        col: 1,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a === b) == (c)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a === b) != (c)": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "a == b;": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "a!=b;": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "(a + b) == c;": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
      "(a + b)  !=  c;": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedNotEqual,
        hint: EqeqeqHint::UseNoteqeq,
      }],
      "((1) )  ==  (2);": [
      {
        col: 0,
        message: EqeqeqMessage::ExpectedEqual,
        hint: EqeqeqHint::UseEqeqeq,
      }],
    }
  }
}
