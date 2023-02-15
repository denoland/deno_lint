// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::swc::ast::TsKeywordTypeKind::TsAnyKeyword;
use deno_ast::view::TsKeywordType;
use deno_ast::SourceRanged;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoExplicitAny;

const CODE: &str = "no-explicit-any";
const MESSAGE: &str = "`any` type is not allowed";
const HINT: &str = "Use a specific type other than `any`";

impl LintRule for NoExplicitAny {
  fn new() -> Arc<Self> {
    Arc::new(NoExplicitAny)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoExplicitAnyHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_explicit_any.md")
  }
}

struct NoExplicitAnyHandler;

impl Handler for NoExplicitAnyHandler {
  fn ts_keyword_type(
    &mut self,
    ts_keyword_type: &TsKeywordType,
    ctx: &mut Context,
  ) {
    if ts_keyword_type.keyword_kind() == TsAnyKeyword {
      ctx.add_diagnostic_with_hint(
        ts_keyword_type.range(),
        CODE,
        MESSAGE,
        HINT,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_explicit_any_valid() {
    assert_lint_ok! {
      NoExplicitAny,
      r#"
class Foo {
  static _extensions: {
    // deno-lint-ignore no-explicit-any
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#,
      r#"
type RequireWrapper = (
  // deno-lint-ignore no-explicit-any
  exports: any,
  // deno-lint-ignore no-explicit-any
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#,
    };
  }

  #[test]
  fn no_explicit_any_invalid() {
    assert_lint_err! {
      NoExplicitAny,
      "function foo(): any { return undefined; }": [{ col: 16, message: MESSAGE, hint: HINT }],
      "function bar(): Promise<any> { return undefined; }": [{ col: 24, message: MESSAGE, hint: HINT }],
      "const a: any = {};": [{ col: 9, message: MESSAGE, hint: HINT }],
      r#"
class Foo {
  static _extensions: {
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#: [{ line: 4, col: 57, message: MESSAGE, hint: HINT }],
      r#"
type RequireWrapper = (
  exports: any,
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#: [{ line: 3, col: 11, message: MESSAGE, hint: HINT }, { line: 4, col: 11, message: MESSAGE, hint: HINT }],
    }
  }
}
