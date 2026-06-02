// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;

#[derive(Debug)]
pub struct FreshHandlerExport;

const CODE: &str = "fresh-handler-export";
const MESSAGE: &str =
  "Fresh middlewares must be exported as \"handler\" but got \"handlers\" instead.";
const HINT: &str = "Did you mean \"handler\"?";

impl LintRule for FreshHandlerExport {
  fn tags(&self) -> Tags {
    &[tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = FreshHandlerExportHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct FreshHandlerExportHandler;

impl Handler<'_> for FreshHandlerExportHandler {
  fn export_named_declaration(
    &mut self,
    export_decl: &ExportNamedDeclaration,
    ctx: &mut Context,
  ) {
    // Fresh only considers components in the routes/ folder to be
    // server components.
    let Some(mut path_segments) = ctx.specifier().path_segments() else {
      return;
    };
    if !path_segments.any(|part| part == "routes") {
      return;
    }

    let Some(decl) = &export_decl.declaration else {
      return;
    };

    let (name, span) = match decl {
      Declaration::VariableDeclaration(var_decl) => {
        if let Some(first) = var_decl.declarations.first() {
          if let BindingPattern::BindingIdentifier(ident) = &first.id {
            (ident.name.as_str(), ident.span)
          } else {
            return;
          }
        } else {
          return;
        }
      }
      Declaration::FunctionDeclaration(fn_decl) => {
        if let Some(id) = &fn_decl.id {
          (id.name.as_str(), id.span)
        } else {
          return;
        }
      }
      _ => return,
    };

    if name == "handlers" {
      ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
    }
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

    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export const handlers = {}"#: [
    {
      col: 13,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export function handlers() {}"#: [
    {
      col: 16,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export async function handlers() {}"#: [
    {
      col: 22,
      message: MESSAGE,
      hint: HINT,
    }]);
  }
}
