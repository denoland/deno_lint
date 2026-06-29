// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};

use deno_ast::view::{Decl, Pat, Program};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct FreshHandlerExport;

const CODE: &str = "fresh-handler-export";
const MESSAGE: &str =
  "Fresh only recognizes \"handler\", \"handlers\", and \"config\" exports in route files.";
const HINT: &str = "Rename this export or move it out of the route file.";

/// Named exports that Fresh recognizes in a route file. A default export (the
/// route component) is a separate AST node (`ExportDefaultDecl`) that this rule
/// never reaches, so it does not need to be listed here.
const ALLOWED_EXPORTS: &[&str] = &["handler", "handlers", "config"];

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
    export_decl: &deno_ast::view::ExportDecl,
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

    let id = match export_decl.decl {
      Decl::Var(var_decl) => {
        if let Some(first) = var_decl.decls.first() {
          let Pat::Ident(name_ident) = first.name else {
            return;
          };
          name_ident.id
        } else {
          return;
        }
      }
      Decl::Fn(fn_decl) => fn_decl.ident,
      _ => return,
    };

    // Flag any export Fresh doesn't recognize in a route file. `config`
    // (`RouteConfig`) is a legitimate route export alongside the handler, so it
    // must be allowed; otherwise valid routes would get a false positive.
    let sym = id.sym();
    if !ALLOWED_EXPORTS.iter().any(|allowed| sym.eq(*allowed)) {
      ctx.add_diagnostic_with_hint(id.range(), CODE, MESSAGE, HINT);
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

    // Both "handler" and "handlers" are valid in Fresh 2.x
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      r#"export const handlers = {}"#,
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      r#"export function handlers() {}"#,
    );
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      r#"export async function handlers() {}"#,
    );

    // `config` (`RouteConfig`) is a recognized Fresh route export and must not
    // be flagged. Regression test for the whitelist false positive.
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      r#"export const config = {}"#,
    );

    // A default export is the route component, which Fresh recognizes; it is a
    // different AST node and is never flagged by this rule.
    assert_lint_ok!(
      FreshHandlerExport,
      filename: "file:///routes/index.tsx",
      r#"export default function Page() {}"#,
    );

    // Unknown export names should be flagged
    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export const foo = {}"#: [
    {
      col: 13,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export function bar() {}"#: [
    {
      col: 16,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshHandlerExport, filename: "file:///routes/index.tsx",  r#"export async function baz() {}"#: [
    {
      col: 22,
      message: MESSAGE,
      hint: HINT,
    }]);
  }
}
