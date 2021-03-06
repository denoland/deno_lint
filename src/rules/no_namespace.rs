// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::{TsModuleDecl, TsModuleName};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoNamespace;

const CODE: &str = "no-namespace";
const MESSAGE: &str = "custom typescript modules are outdated";

impl LintRule for NoNamespace {
  fn new() -> Box<Self> {
    Box::new(NoNamespace)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoNamespaceVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
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

impl<'c> VisitAll for NoNamespaceVisitor<'c> {
  fn visit_ts_module_decl(&mut self, mod_decl: &TsModuleDecl, _: &dyn Node) {
    if !mod_decl.global && !mod_decl.declare {
      if let TsModuleName::Ident(_) = mod_decl.id {
        self.context.add_diagnostic(mod_decl.span, CODE, MESSAGE);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_namespace_valid() {
    assert_lint_ok! {
      NoNamespace,
      r#"declare global {}"#,
      r#"declare module 'foo' {}"#,
      r#"declare module foo {}"#,
      r#"declare namespace foo {}"#,
    };
  }

  #[test]
  fn no_namespace_invalid() {
    assert_lint_err! {
      NoNamespace,
      "module foo {}": [{col: 0, message: MESSAGE }],
      "namespace foo {}": [{col: 0, message: MESSAGE }],
      "namespace Foo.Bar { namespace Baz.Bas {} }": [
        {
          col: 0,
          message: MESSAGE
        },
        {
          col: 20,
          message: MESSAGE
        },
      ],
    };
  }
}
