// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{Tags, WORKSPACE};
use deno_ast::oxc::ast::ast::{
  Expression, ImportDeclaration, ImportExpression, Program,
};

#[derive(Debug)]
pub struct NoImportPrefix;

const CODE: &str = "no-import-prefix";
const MESSAGE: &str =
  "Inline 'npm:', 'jsr:' or 'https:' dependency not allowed";
const HINT: &str = "Add it as a dependency in a deno.json or package.json instead and reference it here via its bare specifier";

impl LintRule for NoImportPrefix {
  fn tags(&self) -> Tags {
    &[WORKSPACE]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoImportPrefixHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoImportPrefixHandler;

impl Handler<'_> for NoImportPrefixHandler {
  fn import_declaration(
    &mut self,
    node: &ImportDeclaration,
    ctx: &mut Context,
  ) {
    if is_non_bare(node.source.value.as_str()) {
      ctx.add_diagnostic_with_hint(node.source.span, CODE, MESSAGE, HINT);
    }
  }

  fn import_expression(
    &mut self,
    node: &ImportExpression,
    ctx: &mut Context,
  ) {
    if let Expression::StringLiteral(lit) = &node.source {
      if is_non_bare(lit.value.as_str()) {
        ctx.add_diagnostic_with_hint(lit.span, CODE, MESSAGE, HINT);
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
