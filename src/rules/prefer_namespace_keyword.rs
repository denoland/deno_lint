// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Program, TSModuleDeclaration, TSModuleDeclarationKind,
  TSModuleDeclarationName,
};

#[derive(Debug)]
pub struct PreferNamespaceKeyword;

const CODE: &str = "prefer-namespace-keyword";
const MESSAGE: &str = "`module` keyword in module declaration is not allowed";

impl LintRule for PreferNamespaceKeyword {
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
    let mut handler = PreferNamespaceKeywordHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct PreferNamespaceKeywordHandler;

impl Handler<'_> for PreferNamespaceKeywordHandler {
  fn ts_module_declaration(
    &mut self,
    mod_decl: &TSModuleDeclaration,
    ctx: &mut Context,
  ) {
    if let TSModuleDeclarationName::StringLiteral(_) = &mod_decl.id {
      return;
    }
    if mod_decl.kind == TSModuleDeclarationKind::Module {
      ctx.add_diagnostic(mod_decl.span, CODE, MESSAGE)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefer_namespace_keyword_valid() {
    assert_lint_ok! {
      PreferNamespaceKeyword,
      "declare module 'foo';",
      "declare module 'foo' {}",
      "namespace foo {}",
      "declare namespace foo {}",
      "declare global {}",
    };
  }

  #[test]
  fn prefer_namespace_keyword_invalid() {
    assert_lint_err! {
      PreferNamespaceKeyword,
      r#"module foo {}"#: [{ col: 0, message: MESSAGE }],
      r#"
      declare module foo {
        declare module bar {}
      }"#: [{ line: 2, col: 6, message: MESSAGE}, { line: 3, col: 8, message: MESSAGE }],
    }
  }
}
