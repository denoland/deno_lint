// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_common::Span;
use swc_ecmascript::ast::{ArrowExpr, Function, Pat};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

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
  fn new() -> Box<Self> {
    Box::new(DefaultParamLast)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = DefaultParamLastVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/default_param_last.md")
  }
}

struct DefaultParamLastVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> DefaultParamLastVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn report(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      DefaultParamLastMessage::DefaultLast,
      DefaultParamLastHint::MoveToEnd,
    );
  }

  fn check_params<'a, 'b, I>(&'a mut self, params: I)
  where
    I: Iterator<Item = &'b Pat>,
  {
    let mut has_seen_normal_param = false;
    for param in params {
      match param {
        Pat::Assign(pat) => {
          if has_seen_normal_param {
            self.report(pat.span);
          }
        }
        Pat::Rest(_) => {}
        _ => {
          has_seen_normal_param = true;
        }
      }
    }
  }
}

impl<'c, 'view> VisitAll for DefaultParamLastVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(function.params.iter().rev().map(|p| &p.pat));
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_params(arrow_expr.params.iter().rev());
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
    }
  }
}
