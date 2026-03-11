// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;
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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = DefaultParamLastHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct DefaultParamLastHandler;

fn has_default(param: &FormalParameter) -> bool {
  param.initializer.is_some()
    || matches!(&param.pattern, BindingPattern::AssignmentPattern(_))
}

impl Handler<'_> for DefaultParamLastHandler {
  fn function(&mut self, function: &Function, ctx: &mut Context) {
    check_params(&function.params, ctx);
  }

  fn arrow_function_expression(
    &mut self,
    arrow_expr: &ArrowFunctionExpression,
    ctx: &mut Context,
  ) {
    check_params(&arrow_expr.params, ctx);
  }
}

fn check_params(params: &FormalParameters, ctx: &mut Context) {
  let mut has_seen_normal_param = false;
  for param in params.items.iter().rev() {
    if has_default(param) {
      if has_seen_normal_param {
        // For constructor params with accessibility (TSParameterProperty),
        // report on the initializer or assignment pattern span
        let report_span = if let BindingPattern::AssignmentPattern(assign) =
          &param.pattern
        {
          assign.span
        } else if let Some(init) = &param.initializer {
          // Report the span covering the pattern + initializer
          let pattern_span = param.pattern.span();
          deno_ast::oxc::span::Span::new(pattern_span.start, init.span().end)
        } else {
          param.span
        };
        ctx.add_diagnostic_with_hint(
          report_span,
          CODE,
          DefaultParamLastMessage::DefaultLast,
          DefaultParamLastHint::MoveToEnd,
        );
      }
    } else {
      // In OXC, rest params are stored separately in FormalParameters.rest,
      // not as items, so all non-default items are normal params.
      has_seen_normal_param = true;
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
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      },
      {
        col: 18,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      }],
      r#"function f(a = 5, b, c = 6, d) {}"#: [
      {
        col: 11,
        message: DefaultParamLastMessage::DefaultLast,
        hint: DefaultParamLastHint::MoveToEnd,
      },
      {
        col: 21,
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
