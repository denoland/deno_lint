// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::{view as ast_view, MediaType, SourceRanged};

#[derive(Debug)]
pub struct NoNamespace;

const CODE: &str = "no-namespace";
const MESSAGE: &str = "TypeScript's `module` and `namespace` are discouraged to
use";
const HINT: &str = "Use ES2015 module syntax (`import`/`export`) to organize
the code instead";

impl LintRule for NoNamespace {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    if matches!(context.media_type(), MediaType::Dts) {
      return;
    }

    NoNamespaceHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_namespace.md")
  }
}

struct NoNamespaceHandler;

impl Handler for NoNamespaceHandler {
  fn ts_module_decl(
    &mut self,
    module_decl: &ast_view::TsModuleDecl,
    ctx: &mut Context,
  ) {
    fn inside_ambient_context(current_node: ast_view::Node) -> bool {
      use deno_ast::view::Node::*;
      match current_node {
        TsModuleDecl(module_decl) if module_decl.declare() => true,
        _ => match current_node.parent() {
          Some(p) => inside_ambient_context(p),
          None => false,
        },
      }
    }

    if !inside_ambient_context(module_decl.as_node()) {
      ctx.add_diagnostic_with_hint(module_decl.range(), CODE, MESSAGE, HINT);
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
      filename: "foo.ts",

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
    };

    assert_lint_ok! {
      NoNamespace,
      filename: "test.d.ts",

      r#"namespace foo {}"#,
      r#"module foo {}"#,

      // https://github.com/denoland/deno_lint/issues/633
      r#"
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
    };
  }

  #[test]
  fn no_namespace_invalid() {
    assert_lint_err! {
      NoNamespace,
      "module foo {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
      ],
      "namespace foo {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "namespace Foo.Bar {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "namespace Foo.Bar { namespace Baz.Bas {} }": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
        {
          col: 20,
          message: MESSAGE,
          hint: HINT,
        },
      ],
    };
  }
}
