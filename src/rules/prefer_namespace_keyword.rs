// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_ecmascript::ast::{TsModuleDecl, TsModuleName};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

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

  fn lint_program(
    &self,
    context: &mut Context,
    program: ProgramRef<'_>,
  ) {
    let mut visitor = PreferNamespaceKeywordVisitor::new(context);
    match program {
        ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
        ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct PreferNamespaceKeywordVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> PreferNamespaceKeywordVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for PreferNamespaceKeywordVisitor<'c> {
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

    let snippet = self
      .context
      .source_map
      .span_to_snippet(mod_decl.span)
      .expect("error in load snippet");

    if let Some(capt) = KEYWORD.captures(&snippet) {
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
