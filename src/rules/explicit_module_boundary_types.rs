// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;

use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, Class, ClassElement, ExportDefaultDeclaration,
  ExportDefaultDeclarationKind, ExportNamedDeclaration, Expression,
  FormalParameter, Function, MethodDefinitionKind, Program, TSType,
  TSTypeAnnotation, VariableDeclaration,
};
use deno_ast::oxc::span::Span;
use deno_ast::MediaType;
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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    // ignore js(x) files
    if matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx) {
      return;
    }
    let mut handler = ExplicitModuleBoundaryTypesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct ExplicitModuleBoundaryTypesHandler;

impl Handler<'_> for ExplicitModuleBoundaryTypesHandler {
  fn export_named_declaration(
    &mut self,
    export_decl: &ExportNamedDeclaration,
    ctx: &mut Context,
  ) {
    let Some(decl) = &export_decl.declaration else {
      return;
    };
    match decl {
      deno_ast::oxc::ast::ast::Declaration::ClassDeclaration(class) => {
        check_class(class, ctx)
      }
      deno_ast::oxc::ast::ast::Declaration::FunctionDeclaration(func) => {
        check_fn(func, ctx, false)
      }
      deno_ast::oxc::ast::ast::Declaration::VariableDeclaration(var) => {
        check_var_decl(var, ctx)
      }
      _ => {}
    }
  }

  fn export_default_declaration(
    &mut self,
    export_default: &ExportDefaultDeclaration,
    ctx: &mut Context,
  ) {
    match &export_default.declaration {
      ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
        check_fn(func, ctx, false)
      }
      ExportDefaultDeclarationKind::ClassDeclaration(class) => {
        check_class(class, ctx)
      }
      ExportDefaultDeclarationKind::ArrowFunctionExpression(arrow) => {
        check_arrow(arrow, ctx)
      }
      _ => {
        // For other expressions like `export default someExpr`, check if it's
        // a function/arrow/class expression
        if let Some(expr) = export_default.declaration.as_expression() {
          check_expr(expr, ctx);
        }
      }
    }
  }
}

fn check_class(class: &Class, ctx: &mut Context) {
  for member in &class.body.body {
    if let ClassElement::MethodDefinition(method) = member {
      let is_setter = method.kind == MethodDefinitionKind::Set;
      check_fn(&method.value, ctx, is_setter);
    }
  }
}

fn check_fn(function: &Function, ctx: &mut Context, is_setter: bool) {
  if !is_setter && function.return_type.is_none() {
    ctx.add_diagnostic_with_hint(
      function.span,
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingRetType,
      ExplicitModuleBoundaryTypesHint::AddRetType,
    );
  }
  for param in &function.params.items {
    check_param(param, ctx);
  }
}

fn check_arrow(arrow: &ArrowFunctionExpression, ctx: &mut Context) {
  if arrow.return_type.is_none() {
    ctx.add_diagnostic_with_hint(
      arrow.span,
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingRetType,
      ExplicitModuleBoundaryTypesHint::AddRetType,
    );
  }
  for param in &arrow.params.items {
    check_param(param, ctx);
  }
}

fn check_ann(ann: Option<&TSTypeAnnotation>, span: Span, ctx: &mut Context) {
  if let Some(ann) = ann {
    if matches!(ann.type_annotation, TSType::TSAnyKeyword(_)) {
      ctx.add_diagnostic_with_hint(
        span,
        CODE,
        ExplicitModuleBoundaryTypesMessage::MissingArgType,
        ExplicitModuleBoundaryTypesHint::AddArgTypes,
      );
    }
  } else {
    ctx.add_diagnostic_with_hint(
      span,
      CODE,
      ExplicitModuleBoundaryTypesMessage::MissingArgType,
      ExplicitModuleBoundaryTypesHint::AddArgTypes,
    );
  }
}

fn check_param(param: &FormalParameter, ctx: &mut Context) {
  // If the parameter has a default value (initializer) and no explicit type annotation,
  // the type can be inferred from the default value — do not require an explicit annotation.
  if param.initializer.is_some() && param.type_annotation.is_none() {
    return;
  }
  check_ann(param.type_annotation.as_deref(), param.span, ctx);
}

fn check_expr(expr: &Expression, ctx: &mut Context) {
  match expr {
    Expression::FunctionExpression(func) => check_fn(func, ctx, false),
    Expression::ArrowFunctionExpression(arrow) => check_arrow(arrow, ctx),
    Expression::ClassExpression(class) => check_class(class, ctx),
    _ => {}
  }
}

fn check_var_decl(var: &VariableDeclaration, ctx: &mut Context) {
  for declarator in &var.declarations {
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
        col: 26,
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
        col: 29,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default class Named { method() { return; } }"#: [
      {
        col: 35,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
    }
  }
}
