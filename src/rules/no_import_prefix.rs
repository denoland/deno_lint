// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{Tags, WORKSPACE};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ImportDecl, Lit};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoImportPrefix;

const CODE: &str = "no-import-prefix";
const MESSAGE: &str = "Inline 'npm:', 'jsr:' or 'https:' dependency discouraged in non-single file projects";
const HINT: &str = "Add it as a dependency in a deno.json or package.json instead and reference it here via its bare specifier";

impl LintRule for NoImportPrefix {
  fn tags(&self) -> Tags {
    &[WORKSPACE]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoImportPrefixHandler.traverse(program, context);
  }
}

struct NoImportPrefixHandler;

impl Handler for NoImportPrefixHandler {
  fn import_decl(&mut self, node: &ImportDecl, ctx: &mut Context) {
    if is_non_bare(node.src.value()) {
      ctx.add_diagnostic_with_hint(node.src.range(), CODE, MESSAGE, HINT);
    }
  }

  fn call_expr(&mut self, node: &CallExpr, ctx: &mut Context) {
    if let Callee::Import(_) = node.callee {
      if let Some(arg) = node.args.first() {
        if let Expr::Lit(Lit::Str(lit)) = arg.expr {
          if is_non_bare(lit.value()) {
            ctx.add_diagnostic_with_hint(arg.range(), CODE, MESSAGE, HINT);
          }
        }
      }
    }
  }
}

fn is_non_bare(s: &str) -> bool {
  s.starts_with("npm:")
    || s.starts_with("jsr:")
    || s.starts_with("http:")
    || s.starts_with("https:")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_with_valid() {
    assert_lint_ok! {
      NoImportPrefix,
      r#"import foo from "foo";"#,
      r#"import foo from "@foo/bar";"#,
      r#"import foo from "./foo";"#,
      r#"import foo from "../foo";"#,
      r#"import foo from "~/foo";"#,
      r#"import("foo")"#,
      r#"import("@foo/bar")"#,
      r#"import("./foo")"#,
      r#"import("../foo")"#,
      r#"import("~/foo")"#,
    }
  }

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoImportPrefix,
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
      r#"import foo from "http://example.com/foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import foo from "https://example.com/foo";"#: [{
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
      r#"import("http://example.com/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT
      }],
      r#"import("https://example.com/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT
      }],
    }
  }
}
