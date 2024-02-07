// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{TsModuleDecl, TsModuleName};
use deno_ast::SourceRanged;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct PreferNamespaceKeyword;

const CODE: &str = "prefer-namespace-keyword";
const MESSAGE: &str = "`module` keyword in module declaration is not allowed";

impl LintRule for PreferNamespaceKeyword {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    PreferNamespaceKeywordHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/prefer_namespace_keyword.md")
  }
}

struct PreferNamespaceKeywordHandler;

impl Handler for PreferNamespaceKeywordHandler {
  fn ts_module_decl(&mut self, mod_decl: &TsModuleDecl, ctx: &mut Context) {
    if let TsModuleName::Str(_) = &mod_decl.id {
      return;
    }
    static KEYWORD: Lazy<Regex> =
      Lazy::new(|| Regex::new(r"(declare\s)?(?P<keyword>\w+)").unwrap());

    let snippet = mod_decl.text_fast(ctx.text_info());
    if let Some(capt) = KEYWORD.captures(snippet) {
      let keyword = capt.name("keyword").unwrap().as_str();
      if keyword == "module" && !mod_decl.global() {
        ctx.add_diagnostic(mod_decl.range(), CODE, MESSAGE)
      }
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
