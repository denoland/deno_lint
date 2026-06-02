// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Expression, Program, TSAsExpression, TSLiteral, TSType, TSTypeAssertion,
  VariableDeclaration,
};
use deno_ast::oxc::span::{GetSpan, Span};
use derive_more::Display;

const CODE: &str = "prefer-as-const";

#[derive(Display)]
enum PreferAsConstMessage {
  #[display(
    fmt = "Expected a `const` assertion instead of a literal type annotation"
  )]
  ExpectedConstAssertion,
}

#[derive(Display)]
enum PreferAsConstHint {
  #[display(fmt = "Remove a literal type annotation and add `as const`")]
  AddAsConst,
}

#[derive(Debug)]
pub struct PreferAsConst;

impl LintRule for PreferAsConst {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = PreferAsConstHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct PreferAsConstHandler;

fn add_diagnostic_helper(span: Span, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    PreferAsConstMessage::ExpectedConstAssertion,
    PreferAsConstHint::AddAsConst,
  );
}

fn compare(
  type_ann: &TSType,
  expr: &Expression,
  span: Span,
  ctx: &mut Context,
) {
  if let TSType::TSLiteralType(lit_type) = type_ann {
    match (&lit_type.literal, expr) {
      (
        TSLiteral::StringLiteral(type_str),
        Expression::StringLiteral(val_str),
      ) => {
        if val_str.value == type_str.value {
          add_diagnostic_helper(span, ctx)
        }
      }
      (
        TSLiteral::NumericLiteral(type_num),
        Expression::NumericLiteral(val_num),
      ) => {
        if (val_num.value - type_num.value).abs() < f64::EPSILON {
          add_diagnostic_helper(span, ctx)
        }
      }
      _ => {}
    }
  }
}

impl Handler<'_> for PreferAsConstHandler {
  fn ts_as_expression(&mut self, as_expr: &TSAsExpression, ctx: &mut Context) {
    compare(
      &as_expr.type_annotation,
      &as_expr.expression,
      as_expr.type_annotation.span(),
      ctx,
    );
  }

  fn ts_type_assertion(
    &mut self,
    type_assertion: &TSTypeAssertion,
    ctx: &mut Context,
  ) {
    compare(
      &type_assertion.type_annotation,
      &type_assertion.expression,
      type_assertion.type_annotation.span(),
      ctx,
    );
  }

  fn variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration,
    ctx: &mut Context,
  ) {
    for decl in &var_decl.declarations {
      if let Some(init) = &decl.init {
        if let Some(type_ann) = &decl.type_annotation {
          compare(
            &type_ann.type_annotation,
            init,
            type_ann.type_annotation.span(),
            ctx,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefer_as_const_valid() {
    assert_lint_ok! {
      PreferAsConst,
      "let foo = 'baz' as const;",
      "let foo = 1 as const;",
      "let foo = { bar: 'baz' as const };",
      "let foo = { bar: 1 as const };",
      "let foo = { bar: 'baz' };",
      "let foo = { bar: 2 };",
      "let foo = <bar>'bar';",
      "let foo = <string>'bar';",
      "let foo = 'bar' as string;",
      "let foo = `bar` as `bar`;",
      "let foo = `bar` as `foo`;",
      "let foo = `bar` as 'bar';",
      "let foo: string = 'bar';",
      "let foo: number = 1;",
      "let foo: 'bar' = baz;",
      "let foo = 'bar';",
      "class foo { bar: 'baz' = 'baz'; }",
      "class foo { bar = 'baz'; }",
      "let foo: 'bar';",
      "let foo = { bar };",
      "let foo: 'baz' = 'baz' as const;",
    };
  }

  #[test]
  fn prefer_as_const_invalid() {
    assert_lint_err! {
      PreferAsConst,
      "let foo = { bar: 'baz' as 'baz' };": [
        {
          col: 26,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = { bar: 1 as 1 };": [
        {
          col: 22,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let [x]: 'bar' = 'bar';": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let {x}: 'bar' = 'bar';": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 'bar' = 'bar';": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 2 = 2;": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 'bar' = 'bar' as 'bar';": [
        {
          col: 26,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = <'bar'>'bar';": [
        {
          col: 11,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = <4>4;": [
        {
          col: 11,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = 'bar' as 'bar';": [
        {
          col: 19,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = 5 as 5;": [
        {
          col: 15,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 1.23456 = 1.23456;": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 2 = 2, bar: 3 = 3;": [
        {
          col: 9,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        },
        {
          col: 21,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],

      // nested
      "let foo = () => { let x: 'x' = 'x'; };": [
        {
          col: 25,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
    };
  }
}
