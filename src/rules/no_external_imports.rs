// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;
use deno_ast::view::ImportDecl;
use deno_ast::{ModuleSpecifier, SourceRanged};
use derive_more::Display;
use std::ffi::OsStr;

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
  #[display(fmt = "Create a deps.ts file or use import maps")]
  CreateDependencyFile,
}

impl LintRule for NoExternalImport {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let mut handler = NoExternalImportHandler;
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
  fn check_import_path(&self, decl: &ImportDecl, ctx: &mut Context) {
    let parsed_src = ModuleSpecifier::parse(decl.src.value());
    let maybe_file_path = ctx.specifier().to_file_path().ok();
    let file_stem = maybe_file_path
      .as_ref()
      .and_then(|p| p.file_stem())
      .and_then(OsStr::to_str);

    if parsed_src.is_ok() && file_stem != Some("deps") {
      ctx.add_diagnostic_with_hint(
        decl.range(),
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

    assert_lint_ok! {
      NoExternalImport,
      filename: if cfg!(windows) {
        "file:///c:/deps.ts"
      } else {
        "file:///deps.ts"
      },
      "import { assertEquals } from 'https://deno.land/std@0.126.0/testing/asserts.ts'"
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
      "import assertEquals from 'http://deno.land/std@0.126.0/testing/asserts.ts'": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
      "import type { Foo } from 'https://example.com';": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
      "import type Foo from 'https://example.com';": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
      "import * as Foo from 'https://example.com';": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
      "import 'https://example.com';": [
        {
          col: 0,
          message: NoExternalImportMessage::Unexpected,
          hint: NoExternalImportHint::CreateDependencyFile,
        },
      ],
    };
  }
}
