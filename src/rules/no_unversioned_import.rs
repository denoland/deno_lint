// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{Tags, RECOMMENDED};
use deno_ast::oxc::ast::ast::{
  Expression, ImportDeclaration, ImportExpression, Program,
};

#[derive(Debug)]
pub struct NoUnversionedImport;

const CODE: &str = "no-unversioned-import";
const MESSAGE: &str = "Missing version in specifier";
const HINT: &str = "Add a version requirement after the package name";

impl LintRule for NoUnversionedImport {
  fn tags(&self) -> Tags {
    &[RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoUnversionedImportHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoUnversionedImportHandler;

impl Handler<'_> for NoUnversionedImportHandler {
  fn import_declaration(
    &mut self,
    node: &ImportDeclaration,
    ctx: &mut Context,
  ) {
    if is_unversioned(node.source.value.as_str()) {
      ctx.add_diagnostic_with_hint(node.source.span, CODE, MESSAGE, HINT);
    }
  }

  fn import_expression(&mut self, node: &ImportExpression, ctx: &mut Context) {
    if let Expression::StringLiteral(lit) = &node.source {
      if is_unversioned(lit.value.as_str()) {
        ctx.add_diagnostic_with_hint(lit.span, CODE, MESSAGE, HINT);
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

fn get_package_req_ref(
  s: &str,
) -> Option<deno_semver::package::PackageReqReference> {
  if let Ok(req_ref) = deno_semver::npm::NpmPackageReqReference::from_str(s) {
    Some(req_ref.into_inner())
  } else if let Ok(req_ref) =
    deno_semver::jsr::JsrPackageReqReference::from_str(s)
  {
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
