// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{
  Program, Statement, TSModuleDeclaration, TSModuleDeclarationBody,
};

#[derive(Debug)]
pub struct NoImplicitDeclareNamespaceExport;

const CODE: &str = "no-implicit-declare-namespace-export";
const MESSAGE: &str =
  "Implicit exports in ambient namespaces are discouraged to
use";
const HINT: &str = "Try adding an `export {};` to the top of the namespace to disable this behavior";

impl LintRule for NoImplicitDeclareNamespaceExport {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoImplicitDeclareNamespaceExportHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoImplicitDeclareNamespaceExportHandler;

impl Handler<'_> for NoImplicitDeclareNamespaceExportHandler {
  fn ts_module_declaration(
    &mut self,
    module_decl: &TSModuleDeclaration,
    ctx: &mut Context,
  ) {
    if module_decl.declare {
      if let Some(TSModuleDeclarationBody::TSModuleBlock(block)) =
        &module_decl.body
      {
        if !block.body.is_empty() {
          // An `export { ... }` (with or without specifiers, but no inline declaration)
          // acts as an explicit export control mechanism in ambient namespaces.
          // e.g. `export {}` disables implicit exports; `export { Y }` means the user
          // controls what is exported. An inline `export type Y = 2` does NOT count.
          let has_specifier_export = block.body.iter().any(|item| {
            if let Statement::ExportNamedDeclaration(e) = item {
              e.declaration.is_none()
            } else {
              false
            }
          });
          let has_non_exported_member = block.body.iter().any(|item| {
            !matches!(
              item,
              Statement::ExportNamedDeclaration(_)
                | Statement::ExportDefaultDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::ImportDeclaration(_)
                | Statement::TSExportAssignment(_)
                | Statement::TSNamespaceExportDeclaration(_)
                | Statement::TSImportEqualsDeclaration(_)
            )
          });
          if !has_specifier_export && has_non_exported_member {
            ctx.add_diagnostic_with_hint(
              module_decl.span,
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
