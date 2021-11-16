// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{
  self as ast_view, Class, ClassMember, Decl, DefaultDecl, Expr,
  TsInterfaceDecl, TsTypeElement,
};

use std::sync::Arc;

#[derive(Debug)]
pub struct RequireJsdoc;

const CODE: &str = "require-jsdoc";
const MESSAGE: &str = "Missing jsdoc comment";

impl LintRule for RequireJsdoc {
  fn new() -> Arc<Self> {
    Arc::new(RequireJsdoc)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    RequireJsdocHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/require_jsdoc.md")
  }
}

struct RequireJsdocHandler;

impl Handler for RequireJsdocHandler {
  fn export_decl(&mut self, _n: &ast_view::ExportDecl, _ctx: &mut Context) {
    match _n.decl {
      Decl::Fn(_) => check_jsdoc(_n, _ctx),
      Decl::TsInterface(interface_decl) => {
        check_jsdoc(_n, _ctx);
        check_interface_decl(interface_decl, _ctx);
      }
      Decl::Class(class_decl) => {
        check_jsdoc(_n, _ctx);
        check_class(class_decl.class, _ctx);
      }
      _ => {}
    }
  }

  fn export_default_decl(
    &mut self,
    _n: &ast_view::ExportDefaultDecl,
    _ctx: &mut Context,
  ) {
    match _n.decl {
      DefaultDecl::Fn(_) => check_jsdoc(_n, _ctx),
      DefaultDecl::Class(class_expr) => {
        check_jsdoc(_n, _ctx);
        check_class(class_expr.class, _ctx);
      }
      DefaultDecl::TsInterfaceDecl(interface_decl) => {
        check_jsdoc(_n, _ctx);
        check_interface_decl(interface_decl, _ctx);
      }
    }
  }

  fn export_default_expr(
    &mut self,
    _n: &ast_view::ExportDefaultExpr,
    _ctx: &mut Context,
  ) {
    if let Expr::Arrow(_) = _n.expr {
      check_jsdoc(_n, _ctx)
    }
  }
}

fn check_class(class: &Class, ctx: &mut Context) {
  class.body.iter().for_each(|member| match member {
    ClassMember::Method(_) | ClassMember::Constructor(_) => {
      check_jsdoc(member, ctx)
    }
    _ => {}
  })
}

fn check_interface_decl(interface_decl: &TsInterfaceDecl, ctx: &mut Context) {
  interface_decl.body.body.iter().for_each(|type_element| {
    if let TsTypeElement::TsMethodSignature(_) = type_element {
      check_jsdoc(type_element, ctx)
    }
  });
}

fn check_jsdoc(n: impl Spanned, ctx: &mut Context) {
  let exists =
    ctx
      .leading_comments_at(n.span().lo)
      .any(|comment| match comment.kind {
        CommentKind::Block => comment.text.starts_with('*'),
        _ => false,
      });
  if !exists {
    ctx.add_diagnostic(n.span(), CODE, MESSAGE);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn require_jsdoc_valid() {
    assert_lint_ok! {
      RequireJsdoc,
      r#"/** doc */ export function a() {}"#,
      r#"/** doc */ export default function() {}"#,
      r#"/** doc */ export class A {
        /** doc */
        constructor() { }
        /** doc */  
        b() {}
        #d() {}
      }"#,
      r#"/** doc */ export interface A {
        /** doc */
        b(): string;
      }"#,
      r#"/** doc */ export default class {
        /** doc */
        constructor() { }
        /** doc */  
        b() {}
        #d() {}
      }"#,
      r#"/** doc */ export default interface A {
        /** doc */
        b(): string;
      }"#,
      r#"/** doc */ export default function a() {}"#,
      r#"/** doc */ export default () => {}"#,
      r#"/** doc */ export default class A {
        /** doc */
        constructor() { }
        /** doc */  
        b() {}
        #d() {}
      }"#,
    };
  }

  #[test]
  fn require_jsdoc_invalid() {
    assert_lint_err! {
      RequireJsdoc,
      r#"export function test() {}"#: [{col: 0, message:MESSAGE}],
      r#"export class A{ constructor(){} b(){}}"#: [
        {col: 0, message:MESSAGE},
        {col:16, message:MESSAGE},
        {col:32, message:MESSAGE}
      ],
      r#"export interface A{ bar(): string;}"#: [
        {col: 0, message:MESSAGE},
        {col: 20, message:MESSAGE},
      ],
      r#"export default function() {}"#: [{col: 0, message:MESSAGE}],
      r#"export default class{ constructor(){} b(){}}"#: [
        {col: 0, message:MESSAGE},
        {col:22, message:MESSAGE},
        {col:38, message:MESSAGE}
      ],
      r#"export default class A{ constructor(){} b(){}};"#: [
        {col: 0, message:MESSAGE},
        {col: 24, message:MESSAGE},
        {col: 40, message:MESSAGE},
      ],
      r#"export default interface A{ bar(): string;}"#: [
        {col: 0, message:MESSAGE},
        {col: 28, message:MESSAGE},
      ],
      r#"export default () => {};"#: [
        {col: 0, message:MESSAGE},
      ],
      r#"export default function a(){};"#: [
        {col: 0, message:MESSAGE},
      ],
    }
  }
}
