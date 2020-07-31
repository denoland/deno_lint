// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{TsModuleDecl, TsModuleName};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoNamespace;

impl LintRule for NoNamespace {
  fn new() -> Box<Self> {
    Box::new(NoNamespace)
  }

  fn code(&self) -> &'static str {
    "no-namespace"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoNamespaceVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoNamespaceVisitor {
  context: Arc<Context>,
}

impl NoNamespaceVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoNamespaceVisitor {
  fn visit_ts_module_decl(
    &mut self,
    mod_decl: &TsModuleDecl,
    parent: &dyn Node,
  ) {
    if !mod_decl.global {
      if let TsModuleName::Ident(_) = mod_decl.id {
        self.context.add_diagnostic(
          mod_decl.span,
          "no-namespace",
          "custom typescript modules are outdated",
        );
      }
    }
    for stmt in &mod_decl.body {
      self.visit_ts_namespace_body(stmt, parent);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_namespace_valid() {
    assert_lint_ok::<NoNamespace>(r#"declare global {}"#);
    assert_lint_ok::<NoNamespace>(r#"declare module 'foo' {}"#);
  }
  #[test]
  fn no_namespace_invalid() {
    assert_lint_err::<NoNamespace>("module foo {}", 0);
    assert_lint_err::<NoNamespace>("declare module foo {}", 0);
    assert_lint_err::<NoNamespace>("namespace foo {}", 0);
    assert_lint_err::<NoNamespace>("declare namespace foo {}", 0);
    assert_lint_err_n::<NoNamespace>(
      "namespace Foo.Bar { namespace Baz.Bas {} }",
      vec![0, 20],
    );
  }
}
