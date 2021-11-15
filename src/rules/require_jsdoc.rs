// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::comments::{Comment, CommentKind};
use deno_ast::swc::common::Spanned;
use deno_ast::view::{self as ast_view, Decl, DefaultDecl};
use std::sync::Arc;

#[derive(Debug)]
pub struct RequireJsdoc;

const CODE: &str = "require-jsdoc";
const MESSAGE: &str = "exported function has no JSDoc";

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
    if let Decl::Fn(_) = _n.decl {
      if !check_jsdoc_exsit(_ctx.leading_comments_at(_n.span().lo)) {
        _ctx.add_diagnostic(_n.span(), CODE, MESSAGE);
      }
    }
  }

  fn export_default_decl(
    &mut self,
    _n: &ast_view::ExportDefaultDecl,
    _ctx: &mut Context,
  ) {
    if let DefaultDecl::Fn(_) = _n.decl {
      if !check_jsdoc_exsit(_ctx.leading_comments_at(_n.span().lo)) {
        _ctx.add_diagnostic(_n.span(), CODE, MESSAGE);
      }
    }
  }
}

fn check_jsdoc_exsit<'c>(
  mut comments: impl Iterator<Item = &'c Comment>,
) -> bool {
  comments.any(|comment| match comment.kind {
    CommentKind::Block => comment.text.starts_with('*'),
    _ => false,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn require_jsdoc_valid() {
    assert_lint_ok! {
      RequireJsdoc,
      r#"/** doc */ export function test() {}"#,
      r#"/** doc */ export default function() {}"#,
    };
  }

  #[test]
  fn require_jsdoc_invalid() {
    assert_lint_err! {
      RequireJsdoc,
      r#"export function test() {}"#: [{col: 0, message:MESSAGE}],
      r#"export default function() {}"#: [{col: 0, message:MESSAGE}],
    }
  }
}
