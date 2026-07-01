// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::rules::config::{RuleConfigError, RuleDef, RuleSeverity};
use crate::Program;
use deno_ast::swc::ast::{BinaryOp, UnaryOp};
use deno_ast::{view as ast_view, SourceRanged};
use derive_more::Display;
use serde::Deserialize;

/// How strictly `eqeqeq` enforces strict equality. Mirrors eslint's `eqeqeq`
/// option string: `"always"` (default) flags every `==`/`!=`; `"smart"` allows
/// them when comparing against `null`, evaluating `typeof`, or comparing two
/// literals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EqeqeqMode {
  #[default]
  Always,
  Smart,
}

#[derive(Debug, Default)]
pub struct Eqeqeq {
  mode: EqeqeqMode,
}

const CODE: &str = "eqeqeq";

fn configure(
  options: Option<&serde_json::Value>,
) -> Result<Box<dyn LintRule>, RuleConfigError> {
  let mode = match options {
    None => EqeqeqMode::default(),
    Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
      RuleConfigError::InvalidOptions {
        code: CODE,
        message: e.to_string(),
      }
    })?,
  };
  Ok(Box::new(Eqeqeq { mode }))
}

impl Eqeqeq {
  /// The rule *definition*: metadata plus the constructor used to build a
  /// configured instance. See [`crate::rules::config`].
  pub fn def() -> RuleDef {
    RuleDef {
      code: CODE,
      tags: &[],
      // Not a recommended rule, so off unless explicitly enabled.
      default_severity: RuleSeverity::Off,
      configure_options: configure,
    }
  }
}

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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    EqeqeqHandler { mode: self.mode }.traverse(program, context);
  }
}

struct EqeqeqHandler {
  mode: EqeqeqMode,
}

/// Whether a `==`/`!=` comparison is permitted under `"smart"` mode: comparing
/// against `null`, evaluating `typeof`, or comparing two literal values.
fn is_smart_allowed(bin_expr: &ast_view::BinExpr) -> bool {
  use ast_view::Expr;
  use ast_view::Lit;

  let is_null = |e: &Expr| matches!(e, Expr::Lit(Lit::Null(_)));
  let is_typeof =
    |e: &Expr| matches!(e, Expr::Unary(u) if u.op() == UnaryOp::TypeOf);
  let is_literal = |e: &Expr| matches!(e, Expr::Lit(_));

  is_null(&bin_expr.left)
    || is_null(&bin_expr.right)
    || is_typeof(&bin_expr.left)
    || is_typeof(&bin_expr.right)
    || (is_literal(&bin_expr.left) && is_literal(&bin_expr.right))
}

impl Handler for EqeqeqHandler {
  fn bin_expr(&mut self, bin_expr: &ast_view::BinExpr, context: &mut Context) {
    if matches!(bin_expr.op(), BinaryOp::EqEq | BinaryOp::NotEq) {
      if self.mode == EqeqeqMode::Smart && is_smart_allowed(bin_expr) {
        return;
      }
      let (message, hint) = if bin_expr.op() == BinaryOp::EqEq {
        (EqeqeqMessage::ExpectedEqual, EqeqeqHint::UseEqeqeq)
      } else {
        (EqeqeqMessage::ExpectedNotEqual, EqeqeqHint::UseNoteqeq)
      };
      context.add_diagnostic_with_hint(bin_expr.range(), CODE, message, hint)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eqeqeq_valid() {
    assert_lint_ok! {
      Eqeqeq::default(),
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
      Eqeqeq::default(),

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

  fn smart() -> Eqeqeq {
    Eqeqeq {
      mode: EqeqeqMode::Smart,
    }
  }

  #[test]
  fn eqeqeq_smart_valid() {
    // `"smart"` permits comparing against null, evaluating typeof, and
    // comparing two literals.
    assert_lint_ok! {
      smart(),
      "a == null",
      "null != a",
      "typeof a == 'number'",
      "'string' != typeof a",
      "true == true",
      "2 == 3",
      "'hello' != 'world'",
    };
  }

  #[test]
  fn eqeqeq_smart_invalid() {
    // Non-null, non-typeof, non-literal comparisons are still flagged.
    assert_lint_err! {
      smart(),
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
    }
  }

  #[test]
  fn eqeqeq_configure_parses_mode() {
    use crate::rules::config::RuleConfigError;

    let always = (Eqeqeq::def().configure_options)(None).unwrap();
    assert_eq!(always.code(), "eqeqeq");

    let smart =
      (Eqeqeq::def().configure_options)(Some(&serde_json::json!("smart")))
        .unwrap();
    assert_eq!(smart.code(), "eqeqeq");

    let err =
      (Eqeqeq::def().configure_options)(Some(&serde_json::json!("nope")))
        .unwrap_err();
    assert!(matches!(
      err,
      RuleConfigError::InvalidOptions { code: "eqeqeq", .. }
    ));
  }
}
