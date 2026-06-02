// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct NoInferrableTypes;

const CODE: &str = "no-inferrable-types";

#[derive(Display)]
enum NoInferrableTypesMessage {
  #[display(fmt = "inferrable types are not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoInferrableTypesHint {
  #[display(fmt = "Remove the type, it is easily inferrable")]
  Remove,
}

impl LintRule for NoInferrableTypes {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoInferrableTypesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoInferrableTypesHandler;

impl NoInferrableTypesHandler {
  fn add_diagnostic_helper(ctx: &mut Context, span: Span) {
    ctx.add_diagnostic_with_hint(
      span,
      CODE,
      NoInferrableTypesMessage::NotAllowed,
      NoInferrableTypesHint::Remove,
    )
  }

  fn check_callee_expr(
    expr: &Expression,
    span: Span,
    expected_sym: &str,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = expr {
      if ident.name.as_str() == expected_sym {
        Self::add_diagnostic_helper(ctx, span);
      }
    }
  }

  fn is_nan_or_infinity(name: &str) -> bool {
    name == "NaN" || name == "Infinity"
  }

  fn get_optional_call_callee<'a>(
    expr: &'a Expression<'a>,
  ) -> Option<&'a Expression<'a>> {
    // In OXC, optional calls are CallExpression with optional=true,
    // wrapped in a ChainExpression
    if let Expression::ChainExpression(chain) = expr {
      if let ChainElement::CallExpression(call) = &chain.expression {
        if call.optional {
          return Some(&call.callee);
        }
      }
    }
    None
  }

  fn check_keyword_type(
    value: &Expression,
    ts_type: &TSType,
    span: Span,
    ctx: &mut Context,
  ) {
    match ts_type {
      TSType::TSBigIntKeyword(_) => match value {
        Expression::BigIntLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::CallExpression(call) => {
          Self::check_callee_expr(&call.callee, span, "BigInt", ctx);
        }
        Expression::UnaryExpression(unary) => match &unary.argument {
          Expression::BigIntLiteral(_) => {
            Self::add_diagnostic_helper(ctx, span);
          }
          Expression::CallExpression(call) => {
            Self::check_callee_expr(&call.callee, span, "BigInt", ctx);
          }
          other => {
            if let Some(callee) = Self::get_optional_call_callee(other) {
              Self::check_callee_expr(callee, span, "BigInt", ctx);
            }
          }
        },
        other => {
          if let Some(callee) = Self::get_optional_call_callee(other) {
            Self::check_callee_expr(callee, span, "BigInt", ctx);
          }
        }
      },
      TSType::TSBooleanKeyword(_) => match value {
        Expression::BooleanLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::CallExpression(call) => {
          Self::check_callee_expr(&call.callee, span, "Boolean", ctx);
        }
        Expression::UnaryExpression(unary) => {
          if unary.operator == UnaryOperator::LogicalNot {
            Self::add_diagnostic_helper(ctx, span);
          }
        }
        other => {
          if let Some(callee) = Self::get_optional_call_callee(other) {
            Self::check_callee_expr(callee, span, "Boolean", ctx);
          }
        }
      },
      TSType::TSNumberKeyword(_) => match value {
        Expression::NumericLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::CallExpression(call) => {
          Self::check_callee_expr(&call.callee, span, "Number", ctx);
        }
        Expression::Identifier(ident) => {
          if Self::is_nan_or_infinity(ident.name.as_str()) {
            Self::add_diagnostic_helper(ctx, span);
          }
        }
        Expression::UnaryExpression(unary) => match &unary.argument {
          Expression::NumericLiteral(_) => {
            Self::add_diagnostic_helper(ctx, span);
          }
          Expression::CallExpression(call) => {
            Self::check_callee_expr(&call.callee, span, "Number", ctx);
          }
          Expression::Identifier(ident) => {
            if Self::is_nan_or_infinity(ident.name.as_str()) {
              Self::add_diagnostic_helper(ctx, span);
            }
          }
          other => {
            if let Some(callee) = Self::get_optional_call_callee(other) {
              Self::check_callee_expr(callee, span, "Number", ctx);
            }
          }
        },
        other => {
          if let Some(callee) = Self::get_optional_call_callee(other) {
            Self::check_callee_expr(callee, span, "Number", ctx);
          }
        }
      },
      TSType::TSNullKeyword(_) => {
        if let Expression::NullLiteral(_) = value {
          Self::add_diagnostic_helper(ctx, span);
        }
      }
      TSType::TSStringKeyword(_) => match value {
        Expression::StringLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::TemplateLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::CallExpression(call) => {
          Self::check_callee_expr(&call.callee, span, "String", ctx);
        }
        other => {
          if let Some(callee) = Self::get_optional_call_callee(other) {
            Self::check_callee_expr(callee, span, "String", ctx);
          }
        }
      },
      TSType::TSSymbolKeyword(_) => {
        if let Expression::CallExpression(call) = value {
          Self::check_callee_expr(&call.callee, span, "Symbol", ctx);
        } else if let Some(callee) = Self::get_optional_call_callee(value) {
          Self::check_callee_expr(callee, span, "Symbol", ctx);
        }
      }
      TSType::TSUndefinedKeyword(_) => match value {
        Expression::Identifier(ident) => {
          if ident.name.as_str() == "undefined" {
            Self::add_diagnostic_helper(ctx, span);
          }
        }
        Expression::UnaryExpression(unary)
          if unary.operator == UnaryOperator::Void =>
        {
          Self::add_diagnostic_helper(ctx, span);
        }
        _ => {}
      },
      _ => {}
    }
  }

  fn check_ref_type(
    value: &Expression,
    ts_type: &TSTypeReference,
    span: Span,
    ctx: &mut Context,
  ) {
    if let TSTypeName::IdentifierReference(ident) = &ts_type.type_name {
      if ident.name.as_str() != "RegExp" {
        return;
      }
      match value {
        Expression::RegExpLiteral(_) => {
          Self::add_diagnostic_helper(ctx, span);
        }
        Expression::CallExpression(call) => {
          Self::check_callee_expr(&call.callee, span, "RegExp", ctx);
        }
        Expression::NewExpression(new_expr) => {
          if let Expression::Identifier(ident) = &new_expr.callee {
            if ident.name.as_str() == "RegExp" {
              Self::add_diagnostic_helper(ctx, span);
            }
          } else if let Some(callee) =
            Self::get_optional_call_callee(&new_expr.callee)
          {
            Self::check_callee_expr(callee, span, "RegExp", ctx);
          }
        }
        other => {
          if let Some(callee) = Self::get_optional_call_callee(other) {
            Self::check_callee_expr(callee, span, "RegExp", ctx);
          }
        }
      }
    }
  }

  fn check_ts_type(
    value: &Expression,
    ts_type_ann: &TSTypeAnnotation,
    span: Span,
    ctx: &mut Context,
  ) {
    match &ts_type_ann.type_annotation {
      TSType::TSTypeReference(ts_type_ref) => {
        Self::check_ref_type(value, ts_type_ref, span, ctx);
      }
      other => {
        Self::check_keyword_type(value, other, span, ctx);
      }
    }
  }
}

impl Handler<'_> for NoInferrableTypesHandler {
  fn variable_declarator(
    &mut self,
    decl: &VariableDeclarator,
    ctx: &mut Context,
  ) {
    if let Some(init) = &decl.init {
      if let BindingPattern::BindingIdentifier(_) = &decl.id {
        if let Some(type_ann) = &decl.type_annotation {
          Self::check_ts_type(init, type_ann, decl.span, ctx);
        }
      }
    }
  }

  fn formal_parameter(&mut self, param: &FormalParameter, ctx: &mut Context) {
    // In OXC, default parameters are represented via FormalParameter.initializer
    if let Some(init) = &param.initializer {
      if let BindingPattern::BindingIdentifier(_) = &param.pattern {
        if let Some(type_ann) = &param.type_annotation {
          Self::check_ts_type(init, type_ann, param.span, ctx);
        }
      }
    }
  }

  fn property_definition(
    &mut self,
    prop: &PropertyDefinition,
    ctx: &mut Context,
  ) {
    if prop.readonly || prop.optional {
      return;
    }
    if let Some(init) = &prop.value {
      if let Some(type_ann) = &prop.type_annotation {
        // Covers both regular and private properties
        Self::check_ts_type(init, type_ann, prop.span, ctx);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_inferrable_types_valid() {
    assert_lint_ok! {
      NoInferrableTypes,
      "const a = 10n",
      "const a = -10n",
      "const a = BigInt(10)",
      "const a = -BigInt?.(10)",
      "const a = +BigInt?.(10)",
      "const a = false",
      "const a = true",
      "const a = Boolean(true)",
      "const a = Boolean(null)",
      "const a = Boolean?.(null)",
      "const a = !0",
      "const a = 10",
      "const a = +10",
      "const a = -10",
      "const a = Number('1')",
      "const a = +Number('1')",
      "const a = -Number('1')",
      "const a = Number?.('1')",
      "const a = +Number?.('1')",
      "const a = -Number?.('1')",
      "const a = Infinity",
      "const a = +Infinity",
      "const a = -Infinity",
      "const a = NaN",
      "const a = +NaN",
      "const a = -NaN",
      "const a = null",
      "const a = /a/",
      "const a = RegExp('a')",
      "const a = RegExp?.('a')",
      "const a = 'str'",
      r#"const a = "str""#,
      "const a = `str`",
      "const a = String(1)",
      "const a = String?.(1)",
      "const a = Symbol('a')",
      "const a = Symbol?.('a')",
      "const a = undefined",
      "const a = void someValue",
      "const fn = (a = 5, b = true, c = 'foo') => {};",
      "const fn = function (a = 5, b = true, c = 'foo') {};",
      "function fn(a = 5, b = true, c = 'foo') {}",
      "function fn(a: number, b: boolean, c: string) {}",
      "class Foo {
      a = 5;
      b = true;
      c = 'foo';
    }",
      "class Foo {
      readonly a: number = 5;
      }",
      "class Foo {
        a?: number = 5;
        b?: boolean = true;
        c?: string = 'foo';
      }",
      "const fn = function (a: any = 5, b: any = true, c: any = 'foo') {};",
    };
  }

  #[test]
  fn no_inferrable_types_invalid() {
    assert_lint_err! {
      NoInferrableTypes,
      "const a: bigint = 10n": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -10n": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = BigInt(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -BigInt?.(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -BigInt?.(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = false": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = true": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean(true)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean(null)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean?.(null)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = !0": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = 10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: null = null": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = /a/": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = RegExp('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = RegExp?.('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = 'str'": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      r#"const a: string = "str""#: [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = `str`": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = String(1)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = String?.(1)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: symbol = Symbol('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: symbol = Symbol?.('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: undefined = undefined": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: undefined = void someValue": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = 0, b: string = 'foo';": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 21,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "function f(a: number = 5) {};": [
        {
          col: 11,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const fn = (a: number = 5, b: boolean = true, c: string = 'foo') => {};": [
        {
          col: 12,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 27,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 46,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a: number = 42; }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a(x: number = 42) {} }": [
        {
          col: 12,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],

      // https://github.com/denoland/deno_lint/issues/558
      "class A { #foo: string = '' }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { static #foo: string = '' }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { #foo(x: number = 42) {} }": [
        {
          col: 15,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { static #foo(x: number = 42) {} }": [
        {
          col: 22,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],

      // nested
      "function a() { const x: number = 5; }": [
        {
          col: 21,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a = () => { const b = (x: number = 42) => {}; };": [
        {
          col: 29,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a = class { b: number = 42; }; }": [
        {
          col: 22,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a = function () { let x: number = 42; };": [
        {
          col: 28,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
    };
  }
}
