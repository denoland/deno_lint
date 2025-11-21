// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct NoImplicitDeclareNamespaceExport;

const CODE: &str = "no-implicit-declare-namespace-export";
const MESSAGE: &str =
  "Implicit exports in ambient namespaces are discouraged to use";
const HINT: &str = "Try adding an `export {};` to the top of the namespace to disable this behavior";

impl LintRule for NoImplicitDeclareNamespaceExport {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoImplicitDeclareNamespaceExportHandler.traverse(program, context);
  }
}

struct NoImplicitDeclareNamespaceExportHandler;

impl Handler for NoImplicitDeclareNamespaceExportHandler {
  fn ts_module_decl(
    &mut self,
    module_decl: &ast_view::TsModuleDecl,
    ctx: &mut Context,
  ) {
    if module_decl.inner.declare {
      if let Some(ast_view::TsNamespaceBody::TsModuleBlock(block)) =
        module_decl.body
      {
        if !block.body.is_empty() {
          let has_named_export = block.body.iter().any(|item| {
            matches!(
              item,
              ast_view::ModuleItem::ModuleDecl(
                ast_view::ModuleDecl::ExportNamed(_)
              )
            )
          });
          let has_non_exported_member = block
            .body
            .iter()
            .any(|item| matches!(item, ast_view::ModuleItem::Stmt(_)));
          if !has_named_export && has_non_exported_member {
            ctx.add_diagnostic_with_hint(
              module_decl.range(),
              CODE,
              MESSAGE,
              HINT,
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn use_explicit_namespace_export_valid() {
    assert_lint_ok! {
      NoImplicitDeclareNamespaceExport,
      filename: "file:///foo.ts",
      r#"
namespace foo {
  type X = 1;
  const FOO = 1;
}
      "#,
      r#"namespace empty {}"#,
      r#"
declare namespace foo {
  export type X = 1;
}
      "#,
      r#"
declare namespace bar {
  export {};
  type X = 1;
}
      "#,
      r#"
declare namespace bar {
  type X = 1;
  type Y = 2;
  export { Y };
}
      "#,
      r#"
declare namespace empty {}
      "#,
    };

    assert_lint_ok! {
      NoImplicitDeclareNamespaceExport,
      filename: "file:///foo.d.ts",

      r#"
declare namespace foo {
  export type X = 1;
}
      "#,
      r#"
declare namespace bar {
  export {};
  type X = 1;
}
      "#,
      r#"
declare namespace bar {
  export {};
  type X = 1;
  export type Y = 2;
}
      "#,
      r#"
declare namespace empty {}
      "#,
    };
  }

  #[test]
  fn use_explicit_namespace_export_invalid() {
    assert_lint_err! {
      NoImplicitDeclareNamespaceExport,
      r#"declare namespace foo { type X = 1; }"#: [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };

    assert_lint_err! {
      NoImplicitDeclareNamespaceExport,
      r#"declare namespace foo { type X = 1; export type Y = 2; }"#: [
        { col: 0,  message: MESSAGE, hint: HINT }
      ],
    };

    assert_lint_err! {
      NoImplicitDeclareNamespaceExport,
      r#"declare namespace foo { interface X {} }"#: [
        { col: 0,  message: MESSAGE, hint: HINT }
      ],
    };

    assert_lint_err! {
      NoImplicitDeclareNamespaceExport,
      r#"declare namespace foo { class X {} }"#: [
        { col: 0,  message: MESSAGE, hint: HINT }
      ],
    };

    assert_lint_err! {
      NoImplicitDeclareNamespaceExport,
      r#"declare namespace foo { enum X { A } }"#: [
        { col: 0,  message: MESSAGE, hint: HINT }
      ],
    };
  }
}
