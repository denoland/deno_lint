// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{Tags, RECOMMENDED};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ImportDecl, Lit};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoUnversionedImport;

const CODE: &str = "no-unversioned-import";
const MESSAGE: &str = "Missing version in specifier";
const HINT: &str = "Add a version at the end";

impl LintRule for NoUnversionedImport {
  fn tags(&self) -> Tags {
    &[RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoUnversionedImportHandler.traverse(program, context);
  }
}

struct NoUnversionedImportHandler;

impl Handler for NoUnversionedImportHandler {
  fn import_decl(&mut self, node: &ImportDecl, ctx: &mut Context) {
    if is_unversioned(node.src.value()) {
      ctx.add_diagnostic_with_hint(node.src.range(), CODE, MESSAGE, HINT);
    }
  }

  fn call_expr(&mut self, node: &CallExpr, ctx: &mut Context) {
    if let Callee::Import(_) = node.callee {
      if let Some(arg) = node.args.first() {
        if let Expr::Lit(Lit::Str(lit)) = arg.expr {
          if is_unversioned(lit.value()) {
            ctx.add_diagnostic_with_hint(arg.range(), CODE, MESSAGE, HINT);
          }
        }
      }
    }
  }
}

fn is_unversioned(s: &str) -> bool {
  if let Some(req_ref) = get_package_req_ref(s) {
    req_ref.req.version_req.version_text() == "*"
  } else {
    false
  }
}

fn get_package_req_ref(s: &str) -> Option<deno_semver::package::PackageReqReference> {
  if let Ok(req_ref) = deno_semver::npm::NpmPackageReqReference::from_str(s) {
    Some(req_ref.into_inner())
  } else if let Ok(req_ref) = deno_semver::jsr::JsrPackageReqReference::from_str(s) {
    Some(req_ref.into_inner())
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_with_valid() {
    assert_lint_ok! {
      NoUnversionedImport,
      r#"import foo from "foo";"#,
      r#"import foo from "@foo/bar";"#,
      r#"import foo from "./foo";"#,
      r#"import foo from "../foo";"#,
      r#"import foo from "~/foo";"#,
      r#"import foo from "npm:foo@1.2.3";"#,
      r#"import foo from "npm:foo@latest";"#,
      r#"import foo from "npm:foo@^1.2.3";"#,
      r#"import foo from "npm:@foo/bar@1.2.3";"#,
      r#"import foo from "npm:@foo/bar@^1.2.3";"#,
      r#"import foo from "jsr:@foo/bar@1.2.3";"#,
      r#"import foo from "jsr:@foo/bar@^1.2.3";"#,
      r#"import("foo")"#,
      r#"import("@foo/bar")"#,
      r#"import("./foo")"#,
      r#"import("../foo")"#,
      r#"import("~/foo")"#,
      r#"import("npm:foo@1.2.3")"#,
      r#"import("npm:foo@^1.2.3")"#,
      r#"import("npm:@foo/bar@1.2.3")"#,
      r#"import("npm:@foo/bar@^1.2.3")"#,
      r#"import("jsr:@foo/bar@1.2.3")"#,
      r#"import("jsr:@foo/bar@^1.2.3")"#,
    }
  }

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoUnversionedImport,
      r#"import foo from "jsr:@foo/foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import foo from "npm:foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import foo from "npm:@foo/bar";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import("jsr:@foo/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import("npm:foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import("npm:@foo/bar");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT
      }],
    }
  }
}
