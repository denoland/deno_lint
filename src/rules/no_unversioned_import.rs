// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ImportDecl, Lit};
use deno_ast::SourceRanged;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct NoUnversionedImport;

const CODE: &str = "no-unversioned-import";
const MESSAGE: &str = "Missing version in specifier";
const HINT: &str = "Add a version at the end";

impl LintRule for NoUnversionedImport {
  fn tags(&self) -> &'static [&'static str] {
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
    NoUnversionedImportHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unversioned_import.md")
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

static NPM_REG: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"^npm:(@.+\/[^@]+|[^@]+)$").unwrap());
static JSR_REG: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"^jsr:@.+\/[^@]+$").unwrap());

fn is_unversioned(s: &str) -> bool {
  if s.starts_with("npm:") {
    return NPM_REG.is_match(s);
  } else if s.starts_with("jsr:") {
    return JSR_REG.is_match(s);
  }

  false
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
