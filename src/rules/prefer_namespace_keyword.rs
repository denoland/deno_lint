// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::scopes::ScopeManager;
use crate::scopes::ScopeVisitor;
use regex::Regex;
use swc_ecma_ast::{Expr, TsModuleDecl, TsModuleName};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct PreferNamespaceKeyword;

impl LintRule for PreferNamespaceKeyword {
  fn new() -> Box<Self> {
    Box::new(PreferNamespaceKeyword)
  }

  fn code(&self) -> &'static str {
    "preferNamespaceKeyword"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(&module, &module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = PreferNamespaceKeywordVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct PreferNamespaceKeywordVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl PreferNamespaceKeywordVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }
}

impl Visit for PreferNamespaceKeywordVisitor {
  fn visit_ts_module_decl(
    &mut self,
    mod_decl: &TsModuleDecl,
    _parent: &dyn Node,
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
      if keyword == "namespace" {
        return;
      }
      self.context.add_diagnostic(
        mod_decl.span,
        "preferNamespaceKeyword",
        "`module` keyword in module decleration is not allowed",
      )
    }

    let scope = self.scope_manager.get_scope_for_span(mod_decl.span);
    println!("{:?}", scope);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn prefer_namespace_keyword_invalid() {
    assert_lint_err::<PreferNamespaceKeyword>(r#"module foo {}"#, 0);
    assert_lint_err_on_line_n::<PreferNamespaceKeyword>(
      r#"
      declare module foo {
        declare module bar {}
      }"#,
      vec![(1, 0), (2, 0)],
    );
  }
}
