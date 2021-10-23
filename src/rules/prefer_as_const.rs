// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::{Span, Spanned};
use deno_ast::view::{
  ArrayPat, BindingIdent, Expr, Lit, ObjectPat, Pat, TsAsExpr, TsLit, TsType,
  TsTypeAnn, TsTypeAssertion, VarDecl,
};
use derive_more::Display;
use std::sync::Arc;

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
  fn new() -> Arc<Self> {
    Arc::new(PreferAsConst)
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
    PreferAsConstHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/prefer_as_const.md")
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

fn compare(type_ann: &TsType, expr: &Expr, span: Span, ctx: &mut Context) {
  if let TsType::TsLitType(lit_type) = &*type_ann {
    if let Expr::Lit(expr_lit) = &*expr {
      match (expr_lit, &lit_type.lit) {
        (Lit::Str(value_literal), TsLit::Str(type_literal)) => {
          if value_literal.value() == type_literal.value() {
            add_diagnostic_helper(span, ctx)
          }
        }
        (Lit::Num(value_literal), TsLit::Number(type_literal)) => {
          if (value_literal.value() - type_literal.value()).abs() < f64::EPSILON
          {
            add_diagnostic_helper(span, ctx)
          }
        }
        _ => {}
      }
    }
  }
}

impl Handler for PreferAsConstHandler {
  fn ts_as_expr(&mut self, as_expr: &TsAsExpr, ctx: &mut Context) {
    compare(
      &as_expr.type_ann,
      &as_expr.expr,
      as_expr.type_ann.span(),
      ctx,
    );
  }

  fn ts_type_assertion(
    &mut self,
    type_assertion: &TsTypeAssertion,
    ctx: &mut Context,
  ) {
    compare(
      &type_assertion.type_ann,
      &type_assertion.expr,
      type_assertion.type_ann.span(),
      ctx,
    );
  }

  fn var_decl(&mut self, var_decl: &VarDecl, ctx: &mut Context) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        if let Pat::Array(ArrayPat { type_ann, .. })
        | Pat::Object(ObjectPat { type_ann, .. })
        | Pat::Ident(BindingIdent { type_ann, .. }) = &decl.name
        {
          if let Some(TsTypeAnn { type_ann, .. }) = &type_ann {
            compare(type_ann, init, type_ann.span(), ctx);
          }
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

      // https://github.com/denoland/deno_lint/issues/567
      "const",
      "let",
      "var",
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
