// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use regex::Regex;
use swc_ecmascript::ast::{TsModuleDecl, TsModuleName};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct PreferNamespaceKeyword;

impl LintRule for PreferNamespaceKeyword {
  fn new() -> Box<Self> {
    Box::new(PreferNamespaceKeyword)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "prefer-namespace-keyword"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = PreferNamespaceKeywordVisitor::new(context);
    visitor.visit_module(module, module);
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
    lazy_static! {
      static ref KEYWORD: Regex =
        Regex::new(r"(declare\s)?(?P<keyword>\w+)").unwrap();
    }

    let snippet = self
      .context
      .source_map
      .span_to_snippet(mod_decl.span)
      .expect("error in load snippet");

    if let Some(capt) = KEYWORD.captures(&snippet) {
      let keyword = capt.name("keyword").unwrap().as_str();
      if keyword == "module" && !mod_decl.global {
        self.context.add_diagnostic(
          mod_decl.span,
          "prefer-namespace-keyword",
          "`module` keyword in module decleration is not allowed",
        )
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
  use crate::test_util::*;

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
    assert_lint_err::<PreferNamespaceKeyword>(r#"module foo {}"#, 0);
    assert_lint_err_on_line_n::<PreferNamespaceKeyword>(
      r#"
      declare module foo {
        declare module bar {}
      }"#,
      vec![(2, 6), (3, 8)],
    );
  }
}
