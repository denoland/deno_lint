// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsInterfaceDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyInterface;

impl LintRule for NoEmptyInterface {
  fn new() -> Box<Self> {
    Box::new(NoEmptyInterface)
  }

  fn code(&self) -> &'static str {
    "no-empty-interface"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEmptyInterfaceVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoEmptyInterfaceVisitor {
  context: Context,
}

impl NoEmptyInterfaceVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyInterfaceVisitor {
  fn visit_ts_interface_decl(
    &mut self,
    interface_decl: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    if interface_decl.body.body.is_empty() {
      self.context.add_diagnostic(
        interface_decl.span,
        "no-empty-interface",
        "Empty interfaces are not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_interface_valid() {
    assert_lint_ok::<NoEmptyInterface>("interface Foo { a: string }");
    assert_lint_ok::<NoEmptyInterface>("interface Foo { a: number }");

    // This is valid because an interface with more than one supertype
    // can be used as a replacement of a union type.
    assert_lint_ok::<NoEmptyInterface>("interface Foo extends Bar, Baz {}");
  }

  #[test]
  fn no_empty_interface_invalid() {
    assert_lint_err::<NoEmptyInterface>("interface Foo {}", 0);
    assert_lint_err::<NoEmptyInterface>("interface Foo extends {}", 0);
    assert_lint_err_on_line::<NoEmptyInterface>(
      r#"
interface Foo {
  a: string;
}

interface Bar extends Foo {}
"#,
      6,
      0,
    );
    assert_lint_err::<NoEmptyInterface>(
      "interface Foo extends Array<number> {}",
      0,
    );
    assert_lint_err::<NoEmptyInterface>(
      "interface Foo extends Array<number | {}> {}",
      0,
    );
    assert_lint_err_on_line::<NoEmptyInterface>(
      r#"
interface Foo {
  a: string;
}

interface Bar extends Array<Foo> {}
"#,
      6,
      0,
    );
    assert_lint_err_on_line::<NoEmptyInterface>(
      r#"
type R = Record<string, unknown>;
interface Foo extends R {}
"#,
      3,
      0,
    );
    assert_lint_err::<NoEmptyInterface>(
      "interface Foo<T> extends Bar<T> {}",
      0,
    );
    assert_lint_err_on_line::<NoEmptyInterface>(
      r#"
declare module FooBar {
  type Baz = typeof baz;
  export interface Bar extends Baz {}
}
"#,
      4,
      9,
    );
  }
}
