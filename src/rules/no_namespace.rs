// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};
use swc_common::Spanned;

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

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    if context.file_name().ends_with(".d.ts") {
      return;
    }

    NoNamespaceHandler.traverse(program, context);
  }
}

struct NoNamespaceHandler;

impl Handler for NoNamespaceHandler {
  fn ts_module_decl(
    &mut self,
    module_decl: &AstView::TsModuleDecl,
    ctx: &mut Context,
  ) {
    fn inside_ambient_context(current_node: AstView::Node) -> bool {
      use AstView::Node::*;
      match current_node {
        TsModuleDecl(module_decl) if module_decl.declare() => true,
        _ => match current_node.parent() {
          Some(p) => inside_ambient_context(p),
          None => false,
        },
      }
    }

    if !inside_ambient_context(module_decl.into_node()) {
      ctx.add_diagnostic(module_decl.span(), CODE, MESSAGE);
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
      r#"
declare global {
  namespace foo {}
}
      "#,
      r#"
declare module foo {
  namespace bar {}
}
      "#,
      r#"
declare global {
  namespace foo {
    namespace bar {}
  }
}
      "#,
      r#"
declare namespace foo {
  namespace bar {
    namespace baz {}
  }
}
      "#,
      {
        src: r#"namespace foo {}"#,
        filename: "test.d.ts",
      },
      {
        src: r#"module foo {}"#,
        filename: "test.d.ts",
      },
      {
        // https://github.com/denoland/deno_lint/issues/633
        src: r#"
export declare namespace Utility {
  export namespace Matcher {
    export type CharSchema<
      T extends string,
      Schema extends string,
      _Rest extends string = T
    > = _Rest extends `${infer $First}${infer $Rest}`
      ? $First extends Schema
        ? CharSchema<T, Schema, $Rest>
        : never
      : "" extends _Rest
      ? T
      : never;
  }
}
        "#,
        filename: "test.d.ts",
      },
    };
  }

  #[test]
  fn no_namespace_invalid() {
    assert_lint_err! {
      NoNamespace,
      "module foo {}": [{ col: 0, message: MESSAGE }],
      "namespace foo {}": [{ col: 0, message: MESSAGE }],
      "namespace Foo.Bar {}": [{ col: 0, message: MESSAGE }],
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
