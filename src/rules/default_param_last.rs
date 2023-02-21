// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::{view as ast_view, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct DefaultParamLast;

const CODE: &str = "default-param-last";

#[derive(Display)]
enum DefaultParamLastMessage {
  #[display(fmt = "default parameters should be at last")]
  DefaultLast,
}

#[derive(Display)]
enum DefaultParamLastHint {
  #[display(
    fmt = "Modify the signatures to move default parameter(s) to the end"
  )]
  MoveToEnd,
}

impl LintRule for DefaultParamLast {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    DefaultParamLastHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/default_param_last.md")
  }
}

struct DefaultParamLastHandler;

impl Handler for DefaultParamLastHandler {
  fn function(&mut self, function: &ast_view::Function, ctx: &mut Context) {
    check_params(function.params.iter().rev().copied().map(|p| p.pat), ctx);
  }

  fn constructor(
    &mut self,
    constructor: &ast_view::Constructor,
    ctx: &mut Context,
  ) {
    check_params(
      constructor.params.iter().rev().copied().map(|p| match p {
        ast_view::ParamOrTsParamProp::TsParamProp(ts_param_prop) => {
          match ts_param_prop.param {
            ast_view::TsParamPropParam::Ident(ident) => {
              ast_view::Pat::Ident(ident)
            }
            ast_view::TsParamPropParam::Assign(assign) => {
              ast_view::Pat::Assign(assign)
            }
          }
        }
        ast_view::ParamOrTsParamProp::Param(param) => param.pat,
      }),
      ctx,
    )
  }

  fn arrow_expr(
    &mut self,
    arrow_expr: &ast_view::ArrowExpr,
    ctx: &mut Context,
  ) {
    check_params(arrow_expr.params.iter().rev().copied(), ctx);
  }
}

fn check_params<'a, 'b, I>(params: I, ctx: &mut Context)
where
  I: Iterator<Item = ast_view::Pat<'b>>,
{
  let mut has_seen_normal_param = false;
  for param in params {
    match param {
      ast_view::Pat::Assign(pat) => {
        if has_seen_normal_param {
          ctx.add_diagnostic_with_hint(
            pat.range(),
            CODE,
            DefaultParamLastMessage::DefaultLast,
            DefaultParamLastHint::MoveToEnd,
          );
        }
      }
      ast_view::Pat::Rest(_) => {}
      _ => {
        has_seen_normal_param = true;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.9.0/tests/lib/rules/default-param-last.js
  // MIT Licensed.

  #[test]
  fn default_param_last_valid() {
    assert_lint_ok! {
      DefaultParamLast,
      "function f() {}",
      "function f(a) {}",
      "function fn(a, b) {}",
      "function f(a = 5) {}",
      "function fn(a = 2, b = 3) {}",
      "function f(a, b = 5) {}",
      "function f(a, b = 5, c = 5) {}",
      "function f(a, b = 5, ...c) {}",
      "const f = () => {}",
      "const f = (a) => {}",
      "const f = (a = 5) => {}",
      "const f = function f() {}",
      "const f = function f(a) {}",
      "const f = function f(a = 5) {}",
      r#"
class Foo {
  bar(a, b = 2) {}
}
      "#,
      r#"
class Foo {
  constructor(a, b = 2) {}
}
      "#,
      r#"
class Foo {
  constructor(readonly a: number, readonly b = 2) {}
}
      "#,
    };
  }

  #[test]
  fn default_param_last_invalid() {
    assert_lint_err! {
      DefaultParamLast,

      r#"function f(a = 2, b) {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = function (a = 2, b) {}"#: [
      {
        col: 20,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"function f(a = 5, b = 6, c) {}"#: [
      {
        col: 18,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      },
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"function f(a = 5, b, c = 6, d) {}"#: [
      {
        col: 21,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      },
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"function f(a = 5, b, c = 5) {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = (a = 5, b, ...c) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = function f (a, b = 5, c) {}"#: [
      {
        col: 25,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = (a = 5, { b }) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = ({ a } = {}, b) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = ({ a, b } = { a: 1, b: 2 }, c) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = ([a] = [], b) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"const f = ([a, b] = [1, 2], c) => {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],

      r#"
class Foo {
  bar(a = 2, b) {}
}
      "#: [
      {
        line: 3,
        col: 6,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
class Foo {
  constructor(a = 2, b) {}
}
      "#: [
      {
        line: 3,
        col: 14,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
class Foo {
  constructor(readonly a = 2, readonly b: number) {}
}
      "#: [
      {
        line: 3,
        col: 23,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
function f() {
  function g(a = 5, b) {}
}
"#: [
      {
        line: 3,
        col: 13,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
const f = () => {
  function g(a = 5, b) {}
}
"#: [
      {
        line: 3,
        col: 13,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
function f() {
  const g = (a = 5, b) => {}
}
"#: [
      {
        line: 3,
        col: 13,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
const f = () => {
  const g = (a = 5, b) => {}
}
"#: [
      {
        line: 3,
        col: 13,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
class Foo {
  bar(a, b = 1) {
    class X {
      y(c = 3, d) {}
    }
  }
}
"#: [
      {
        line: 5,
        col: 8,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"
class Foo {
  constructor(a, b = 1) {
    class X {
      constructor(c = 3, d) {}
    }
  }
}
"#: [
      {
        line: 5,
        col: 18,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
    }
  }
}
