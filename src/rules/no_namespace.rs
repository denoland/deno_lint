// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{TsModuleDecl, TsModuleName};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoNamespace;

impl LintRule for NoNamespace {
  fn new() -> Box<Self> {
    Box::new(NoNamespace)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-namespace"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoNamespaceVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoNamespaceVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoNamespaceVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoNamespaceVisitor<'c> {
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
    assert_lint_ok! {
      NoNamespace,
      r#"declare global {}"#,
      r#"declare module 'foo' {}"#,
    };
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
