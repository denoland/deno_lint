// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::{Program, ProgramRef};
use deno_ast::swc::ast::{BinaryOp};
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use deno_ast::swc::common::Spanned;
use deno_ast::view as ast_view;
use derive_more::Display;

#[derive(Debug)]
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
    _context: &mut Context<'view>,
    _program: ProgramRef<'view>,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    EqeqeqHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/eqeqeq.md")
  }
}

struct EqeqeqHandler;

impl Handler for EqeqeqHandler {
  fn bin_expr(&mut self, bin_expr: &ast_view::BinExpr, context: &mut Context) {
    if matches!(bin_expr.op(), BinaryOp::EqEq | BinaryOp::NotEq) {
      let (message, hint) = if bin_expr.op() == BinaryOp::EqEq {
        (EqeqeqMessage::ExpectedEqual, EqeqeqHint::UseEqeqeq)
      } else {
        (EqeqeqMessage::ExpectedNotEqual, EqeqeqHint::UseNoteqeq)
      };
      context
        .add_diagnostic_with_hint(bin_expr.span(), CODE, message, hint)
    }
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
