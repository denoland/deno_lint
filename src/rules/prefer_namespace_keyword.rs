// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{TsModuleDecl, TsModuleName};
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct PreferNamespaceKeyword;

const CODE: &str = "prefer-namespace-keyword";
const MESSAGE: &str = "`module` keyword in module decleration is not allowed";

impl LintRule for PreferNamespaceKeyword {
  fn new() -> Box<Self> {
    Box::new(PreferNamespaceKeyword)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = PreferNamespaceKeywordVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/prefer_namespace_keyword.md")
  }
}

struct PreferNamespaceKeywordVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> PreferNamespaceKeywordVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for PreferNamespaceKeywordVisitor<'c, 'view> {
  fn visit_ts_module_decl(
    &mut self,
    mod_decl: &TsModuleDecl,
    parent: &dyn Node,
  ) {
    if let TsModuleName::Str(_) = &mod_decl.id {
      return;
    }
    static KEYWORD: Lazy<Regex> =
      Lazy::new(|| Regex::new(r"(declare\s)?(?P<keyword>\w+)").unwrap());

    let snippet = self.context.file_text_substring(&mod_decl.span);
    if let Some(capt) = KEYWORD.captures(snippet) {
      let keyword = capt.name("keyword").unwrap().as_str();
      if keyword == "module" && !mod_decl.global {
        self.context.add_diagnostic(mod_decl.span, CODE, MESSAGE)
      }
    }
    for stmt in &mod_decl.body {
      self.visit_ts_namespace_body(stmt, parent)
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
