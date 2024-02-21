// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};

use deno_ast::{view as ast_view, MediaType, SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct ExplicitModuleBoundaryTypes;

const CODE: &str = "explicit-module-boundary-types";

#[derive(Display)]
enum ExplicitModuleBoundaryTypesMessage {
  #[display(fmt = "Missing return type on function")]
  MissingRetType,

  #[display(fmt = "All arguments should be typed")]
  MissingArgType,
}

#[derive(Display)]
enum ExplicitModuleBoundaryTypesHint {
  #[display(fmt = "Add a return type to the function signature")]
  AddRetType,

  #[display(fmt = "Add types to all the function arguments")]
  AddArgTypes,
}

impl LintRule for ExplicitModuleBoundaryTypes {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: ast_view::Program,
  ) {
    // ignore js(x) files
    if matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx) {
      return;
    }
    ExplicitModuleBoundaryTypesHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/explicit_module_boundary_types.md")
  }
}

struct ExplicitModuleBoundaryTypesHandler;

impl Handler for ExplicitModuleBoundaryTypesHandler {
  fn export_decl(
    &mut self,
    export_decl: &ast_view::ExportDecl,
    ctx: &mut Context,
  ) {
    use ast_view::Decl;
    match &export_decl.decl {
      Decl::Class(decl) => check_class(decl.class, ctx),
      Decl::Fn(decl) => check_fn(decl.function, ctx, false),
      Decl::Var(var) => check_var_decl(var, ctx),
      _ => {}
    }
  }

  fn export_default_decl(
    &mut self,
    export_default_decl: &ast_view::ExportDefaultDecl,
    ctx: &mut Context,
  ) {
    use ast_view::DefaultDecl;
    match &export_default_decl.decl {
      DefaultDecl::Class(expr) => check_class(expr.class, ctx),
      DefaultDecl::Fn(expr) => check_fn(expr.function, ctx, false),
      _ => {}
    }
  }

  fn export_default_expr(
    &mut self,
    export_default_expr: &ast_view::ExportDefaultExpr,
    ctx: &mut Context,
  ) {
    check_expr(&export_default_expr.expr, ctx);
  }
}

fn check_class(class: &ast_view::Class, ctx: &mut Context) {
  for member in class.body {
    if let ast_view::ClassMember::Method(method) = member {
      let is_setter = method.inner.kind == ast_view::MethodKind::Setter;
      check_fn(method.function, ctx, is_setter);
    }
  }
}

fn check_fn(function: &ast_view::Function, ctx: &mut Context, is_setter: bool) {
  if !is_setter && function.return_type.is_none() {
    ctx.add_diagnostic_with_hint(
      function.range(),
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingRetType,
      ExplicitModuleBoundaryTypesHint::AddRetType,
    );
  }
  for param in function.params {
    check_pat(&param.pat, ctx);
  }
}

fn check_arrow(arrow: &ast_view::ArrowExpr, ctx: &mut Context) {
  if arrow.return_type.is_none() {
    ctx.add_diagnostic_with_hint(
      arrow.range(),
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingRetType,
      ExplicitModuleBoundaryTypesHint::AddRetType,
    );
  }
  for pat in arrow.params {
    check_pat(pat, ctx);
  }
}

fn check_ann(
  ann: Option<&ast_view::TsTypeAnn>,
  range: SourceRange,
  ctx: &mut Context,
) {
  if let Some(ann) = ann {
    if let ast_view::TsType::TsKeywordType(keyword_type) = ann.type_ann {
      if ast_view::TsKeywordTypeKind::TsAnyKeyword
        == keyword_type.keyword_kind()
      {
        ctx.add_diagnostic_with_hint(
          range,
          CODE,
          ExplicitModuleBoundaryTypesMessage::MissingArgType,
          ExplicitModuleBoundaryTypesHint::AddArgTypes,
        );
      }
    }
  } else {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingArgType,
      ExplicitModuleBoundaryTypesHint::AddArgTypes,
    );
  }
}

fn check_pat(pat: &ast_view::Pat, ctx: &mut Context) {
  match pat {
    ast_view::Pat::Ident(ident) => {
      check_ann(ident.type_ann, ident.id.range(), ctx)
    }
    ast_view::Pat::Array(array) => {
      check_ann(array.type_ann, array.range(), ctx)
    }
    ast_view::Pat::Rest(rest) => check_ann(rest.type_ann, rest.range(), ctx),
    ast_view::Pat::Object(object) => {
      check_ann(object.type_ann, object.range(), ctx)
    }
    _ => {}
  };
}

fn check_expr(expr: &ast_view::Expr, ctx: &mut Context) {
  match expr {
    ast_view::Expr::Fn(func) => check_fn(func.function, ctx, false),
    ast_view::Expr::Arrow(arrow) => check_arrow(arrow, ctx),
    ast_view::Expr::Class(class) => check_class(class.class, ctx),
    _ => {}
  }
}

fn check_var_decl(var: &ast_view::VarDecl, ctx: &mut Context) {
  for declarator in var.decls {
    if let Some(expr) = &declarator.init {
      check_expr(expr, ctx)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn explicit_module_boundary_types_valid() {
    assert_lint_ok! {
      ExplicitModuleBoundaryTypes,
      filename: "file:///foo.ts",
      "function test() { return }",
      "export var fn = function (): number { return 1; }",
      "export var arrowFn = (arg: string): string => `test ${arg}`",
      "export var arrowFn = (arg: unknown): string => `test ${arg}`",
      "class Test { method() { return; } }",
      "export function test(arg = 1) : number { return arg;}",
      "export function test(arg :number = 1) : number { return arg;}",
      "export class Test { set method() { return true; } }",
    };

    assert_lint_ok! {
      ExplicitModuleBoundaryTypes,
      filename: "file:///foo.js",
      "function test() { return }",
      "export var fn = function () { return 1; }",
      "export var arrowFn = (arg) => `test ${arg}`",
      "export var arrowFn = (arg) => `test ${arg}`",
      "class Test { method() { return; } }",
    };

    assert_lint_ok! {
      ExplicitModuleBoundaryTypes,
      filename: "file:///foo.jsx",
      "export function Foo(props) {return <div>{props.name}</div>}",
      "export default class Foo { render() { return <div></div>}}"
    };
  }

  #[test]
  fn explicit_module_boundary_types_invalid() {
    assert_lint_err! {
      ExplicitModuleBoundaryTypes,

      r#"export function test() { return; }"#: [
      {
        col: 7,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default function () { return 1; }"#: [
      {
        col: 15,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export var arrowFn = () => 'test';"#: [
      {
        col: 21,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export var arrowFn = (arg): string => `test ${arg}`;"#: [
      {
        col: 22,
        message: ExplicitModuleBoundaryTypesMessage::MissingArgType,
        hint: ExplicitModuleBoundaryTypesHint::AddArgTypes,
      }],
      r#"export var arrowFn = (arg: any): string => `test ${arg}`;"#: [
      {
        col: 22,
        message: ExplicitModuleBoundaryTypesMessage::MissingArgType,
        hint: ExplicitModuleBoundaryTypesHint::AddArgTypes,
      }],
      r#"export class Test { method() { return; } }"#: [
      {
        col: 20,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default () => true;"#: [
      {
        col: 15,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default function() { return true; }"#: [
      {
        col: 15,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default function named() { return true; }"#: [
      {
        col: 15,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default class { method() { return; } }"#: [
      {
        col: 23,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default class Named { method() { return; } }"#: [
      {
        col: 29,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
    }
  }
}
