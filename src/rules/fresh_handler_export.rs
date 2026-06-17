// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};

use deno_ast::view::Program;

#[derive(Debug)]
pub struct FreshHandlerExport;

const CODE: &str = "fresh-handler-export";

impl LintRule for FreshHandlerExport {
  fn tags(&self) -> Tags {
    &[tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    Visitor.traverse(program, context);
  }
}

struct Visitor;

impl Handler for Visitor {
  fn export_decl(
    &mut self,
    _export_decl: &deno_ast::view::ExportDecl,
    _ctx: &mut Context,
  ) {
    // Fresh accepts both "handler" and "handlers" exports
    // No diagnostic needed - both are valid
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fresh_handler_export_name() {
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///foo.jsx",
      "const handler = {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///foo.jsx",
      "function handler() {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///foo.jsx",
      "export const handler = {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///foo.jsx",
      "export const handlers = {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///foo.jsx",
      "export function handlers() {}",
    );

    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/foo.jsx",
      "export const handler = {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/foo.jsx",
      "export function handler() {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/foo.jsx",
      "export async function handler() {}",
    );

    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      "export const handlers = {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      "export function handlers() {}",
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      "export async function handlers() {}",
    );
  }
}
