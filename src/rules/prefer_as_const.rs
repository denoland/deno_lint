// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use derive_more::Display;
use swc_ecmascript::ast::{
  ArrayPat, Expr, Ident, Lit, ObjectPat, Pat, TsAsExpr, TsLit, TsType,
  TsTypeAssertion, VarDecl,
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

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

pub struct PreferAsConst;

impl LintRule for PreferAsConst {
  fn new() -> Box<Self> {
    Box::new(PreferAsConst)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = PreferAsConstVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct PreferAsConstVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> PreferAsConstVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic_helper(&mut self, span: swc_common::Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      PreferAsConstMessage::ExpectedConstAssertion,
      PreferAsConstHint::AddAsConst,
    );
  }

  fn compare(
    &mut self,
    type_ann: &TsType,
    expr: &Expr,
    span: swc_common::Span,
  ) {
    if let TsType::TsLitType(lit_type) = &*type_ann {
      if let Expr::Lit(expr_lit) = &*expr {
        match (expr_lit, &lit_type.lit) {
          (Lit::Str(value_literal), TsLit::Str(type_literal)) => {
            if value_literal.value == type_literal.value {
              self.add_diagnostic_helper(span)
            }
          }
          (Lit::Num(value_literal), TsLit::Number(type_literal)) => {
            let error = 0.01f64;
            if (value_literal.value - type_literal.value).abs() < error {
              self.add_diagnostic_helper(span)
            }
          }
          _ => return,
        }
      }
    }
  }
}

impl<'c> Visit for PreferAsConstVisitor<'c> {
  fn visit_ts_as_expr(&mut self, as_expr: &TsAsExpr, _parent: &dyn Node) {
    self.compare(&as_expr.type_ann, &as_expr.expr, as_expr.span);
  }

  fn visit_ts_type_assertion(
    &mut self,
    type_assertion: &TsTypeAssertion,
    _parent: &dyn Node,
  ) {
    self.compare(
      &type_assertion.type_ann,
      &type_assertion.expr,
      type_assertion.span,
    );
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        match &**init {
          Expr::TsAs(as_expr) => {
            self.visit_ts_as_expr(&as_expr, _parent);
            return;
          }
          Expr::Object(object) => {
            self.visit_object_lit(&object, _parent);
            return;
          }
          Expr::TsTypeAssertion(type_assert) => {
            self.visit_ts_type_assertion(&type_assert, _parent);
            return;
          }
          _ => {}
        }

        if let Pat::Array(ArrayPat { type_ann, .. })
        | Pat::Object(ObjectPat { type_ann, .. })
        | Pat::Ident(Ident { type_ann, .. }) = &decl.name
        {
          if let Some(swc_ecmascript::ast::TsTypeAnn { type_ann, .. }) =
            &type_ann
          {
            self.compare(type_ann, &init, var_decl.span);
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
          col: 17,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = { bar: 1 as 1 };": [
        {
          col: 17,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let [x]: 'bar' = 'bar';": [
        {
          col: 0,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let {x}: 'bar' = 'bar';": [
        {
          col: 0,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 'bar' = 'bar';": [
        {
          col: 0,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 2 = 2;": [
        {
          col: 0,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo: 'bar' = 'bar' as 'bar';": [
        {
          col: 17,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = <'bar'>'bar';": [
        {
          col: 10,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = <4>4;": [
        {
          col: 10,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = 'bar' as 'bar';": [
        {
          col: 10,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
      "let foo = 5 as 5;": [
        {
          col: 10,
          message: PreferAsConstMessage::ExpectedConstAssertion,
          hint: PreferAsConstHint::AddAsConst,
        }
      ],
    };
  }
}
