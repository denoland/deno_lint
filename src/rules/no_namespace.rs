// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::MediaType;

#[derive(Debug)]
pub struct NoNamespace;

const CODE: &str = "no-namespace";
const MESSAGE: &str = "TypeScript's `module` and `namespace` are discouraged to
use";
const HINT: &str = "Use ES2015 module syntax (`import`/`export`) to organize
the code instead";

impl LintRule for NoNamespace {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    if matches!(context.media_type(), MediaType::Dts) {
      return;
    }

    let mut visitor = NoNamespaceVisitor {
      context,
      ambient_depth: 0,
      in_qualified_name_part: false,
    };
    visitor.visit_program(program);
  }
}

struct NoNamespaceVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  /// Track how deep we are inside ambient (declare) contexts.
  ambient_depth: u32,
  /// True if we're currently traversing the inner part of a qualified name
  /// like `Bar` in `namespace Foo.Bar {}`. The inner part should not report.
  in_qualified_name_part: bool,
}

impl<'a> Visit<'a> for NoNamespaceVisitor<'_, 'a> {
  fn visit_ts_global_declaration(
    &mut self,
    global_decl: &TSGlobalDeclaration<'a>,
  ) {
    // `declare global {}` is always valid - it's a global augmentation.
    // We increment ambient_depth so nested namespace declarations are allowed.
    self.ambient_depth += 1;
    walk::walk_ts_global_declaration(self, global_decl);
    self.ambient_depth -= 1;
  }

  fn visit_ts_module_declaration(
    &mut self,
    module_decl: &TSModuleDeclaration<'a>,
  ) {
    let is_qualified_name_part = matches!(
      &module_decl.body,
      Some(TSModuleDeclarationBody::TSModuleDeclaration(_))
    );

    if module_decl.declare {
      // This is a `declare module/namespace` - it's ambient, don't report it.
      self.ambient_depth += 1;
      walk::walk_ts_module_declaration(self, module_decl);
      self.ambient_depth -= 1;
    } else if self.in_qualified_name_part {
      // This is an inner part of a qualified name (e.g., `Bar` in `Foo.Bar`).
      // Don't report it, but DO walk the body normally (not as ambient).
      if is_qualified_name_part {
        // Still in qualified name (e.g., `Baz` in `Foo.Bar.Baz`)
        walk::walk_ts_module_declaration(self, module_decl);
      } else {
        // End of qualified name - the body is a real block, walk normally
        let prev = self.in_qualified_name_part;
        self.in_qualified_name_part = false;
        walk::walk_ts_module_declaration(self, module_decl);
        self.in_qualified_name_part = prev;
      }
    } else if self.ambient_depth == 0 {
      // Not inside any ambient context - this is a violation.
      self.context.add_diagnostic_with_hint(
        module_decl.span,
        CODE,
        MESSAGE,
        HINT,
      );
      if is_qualified_name_part {
        // OXC represents `namespace Foo.Bar {}` as nested `TSModuleDeclaration`
        // nodes, but we only want to report the outermost one. Walk the inner
        // qualified name parts without reporting.
        self.in_qualified_name_part = true;
        walk::walk_ts_module_declaration(self, module_decl);
        self.in_qualified_name_part = false;
      } else {
        walk::walk_ts_module_declaration(self, module_decl);
      }
    } else {
      // Inside an ambient context - this is fine, just walk.
      walk::walk_ts_module_declaration(self, module_decl);
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
      filename: "file:///foo.ts",

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
      filename: "file:///test.d.ts",

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
