// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{BinaryExpression, BinaryOperator, Program};
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
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = EqeqeqHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct EqeqeqHandler;

impl Handler<'_> for EqeqeqHandler {
  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    context: &mut Context,
  ) {
    if matches!(
      bin_expr.operator,
      BinaryOperator::Equality | BinaryOperator::Inequality
    ) {
      let (message, hint) = if bin_expr.operator == BinaryOperator::Equality {
        (EqeqeqMessage::ExpectedEqual, EqeqeqHint::UseEqeqeq)
      } else {
        (EqeqeqMessage::ExpectedNotEqual, EqeqeqHint::UseNoteqeq)
      };
      context.add_diagnostic_with_hint(bin_expr.span, CODE, message, hint)
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
