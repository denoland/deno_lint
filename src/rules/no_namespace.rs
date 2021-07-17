// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse, TraverseFlow};
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};
use swc_common::Spanned;

pub struct NoNamespace;

const CODE: &str = "no-namespace";
const MESSAGE: &str = "TypeScript's `module` and `namespace` are discouraged to
use";
const HINT: &str = "Use ES2015 module syntax (`import`/`export`) to organize
the code instead";

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

  fn docs(&self) -> &'static str {
    r#"Disallows the use of `namespace` and `module` keywords in TypeScript code.

`namespace` and `module` are both thought of as outdated keywords to organize
the code. Instead, it is generally preferable to use ES2015 module syntax (e.g.
`import`/`export`).

However, this rule still allows the use of these keywords in the following two
cases:

- they are used for defining ["ambient" namespaces] along with `declare` keywords
- they are written in TypeScript's type definition files: `.d.ts`

["ambient" namespaces]: https://www.typescriptlang.org/docs/handbook/namespaces.html#ambient-namespaces

### Invalid:

```typescript
// foo.ts
module mod {}
namespace ns {}
```

```dts
// bar.d.ts
// all usage of `module` and `namespace` keywords are allowed in `.d.ts`
```

### Valid:
```typescript
// foo.ts
declare global {}
declare module mod1 {}
declare module "mod2" {}
declare namespace ns {}
```

```dts
// bar.d.ts
module mod1 {}
namespace ns1 {}
declare global {}
declare module mod2 {}
declare module "mod3" {}
declare namespace ns2 {}
```
"#
  }
}

struct NoNamespaceHandler;

impl Handler for NoNamespaceHandler {
  fn ts_module_decl(
    &mut self,
    module_decl: &AstView::TsModuleDecl,
    ctx: &mut Context,
  ) -> TraverseFlow {
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

    if !inside_ambient_context(module_decl.as_node()) {
      ctx.add_diagnostic_with_hint(module_decl.span(), CODE, MESSAGE, HINT);
    }

    TraverseFlow::Continue
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
