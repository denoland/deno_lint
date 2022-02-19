// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view::ImportDecl;
use derive_more::Display;
use std::sync::Arc;
use url::Url;

#[derive(Debug)]
pub struct NoExternalImport;

const CODE: &str = "no-external-import";

#[derive(Display)]
enum NoExternalImportMessage {
  #[display(fmt = "Not allowed to import external resources")]
  Unexpected,
}

#[derive(Display)]
enum NoExternalImportHint {
  #[display(fmt = "Create a mod.ts file and use import maps there")]
  CreateDependencyFile,
}

impl LintRule for NoExternalImport {
  fn new() -> Arc<Self> {
    Arc::new(NoExternalImport)
  }

  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let mut handler = NoExternalImportHandler::default();
    handler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_external_imports.md")
  }
}

#[derive(Default)]
struct NoExternalImportHandler;

impl NoExternalImportHandler {
  fn check_import_path<'a>(&'a self, decl: &ImportDecl, ctx: &mut Context) {
    let parsed_src = Url::parse(decl.src.value());
    if parsed_src.is_ok() {
      ctx.add_diagnostic_with_hint(
        decl.span(),
        CODE,
        NoExternalImportMessage::Unexpected,
        NoExternalImportHint::CreateDependencyFile,
      );
    }
  }
}

impl Handler for NoExternalImportHandler {
  fn import_decl(
    &mut self,
    decl: &deno_ast::view::ImportDecl,
    ctx: &mut Context,
  ) {
    self.check_import_path(decl, ctx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_external_import_valid() {
    assert_lint_ok! {
      NoExternalImport,
      "import { assertEquals } from './deps.ts'",
      "import { assertEquals } from 'deps.ts'",
      "import Foo from './deps.ts';",
      "import type { Foo } from './deps.ts';",
      "import type Foo from './deps.ts';",
      "import * as Foo from './deps.ts';",
      "import './deps.ts';",
      "const foo = await import('https://example.com');"
    };
  }

  #[test]
  fn no_external_import_invalid() {
    assert_lint_err! {
      NoExternalImport,
      "import { assertEquals } from 'https://deno.land/std@0.126.0/testing/asserts.ts'": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
    };
  }
}
