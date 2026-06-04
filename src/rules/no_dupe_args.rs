// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, BindingPattern, FormalParameter, Function, Program,
};
use deno_ast::oxc::span::Span;
use derive_more::Display;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug)]
pub struct NoDupeArgs;

const CODE: &str = "no-dupe-args";

#[derive(Display)]
enum NoDupeArgsMessage {
  #[display(fmt = "Duplicate arguments not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoDupeArgsHint {
  #[display(fmt = "Rename or remove the duplicate (e.g. same name) argument")]
  RenameOrRemove,
}

impl LintRule for NoDupeArgs {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    "no-dupe-args"
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoDupeArgsHandler::default();
    crate::handler::traverse_program(&mut handler, program, context);
    handler.report_errors(context);
  }
}

#[derive(Default)]
struct NoDupeArgsHandler {
  /// Accumulated errors to report
  error_ranges: BTreeSet<Span>,
}

impl NoDupeArgsHandler {
  fn report_errors(self, ctx: &mut Context) {
    for range in &self.error_ranges {
      ctx.add_diagnostic_with_hint(
        *range,
        CODE,
        NoDupeArgsMessage::Unexpected,
        NoDupeArgsHint::RenameOrRemove,
      );
    }
  }

  fn check_params<'a, I>(&mut self, range: Span, params: I)
  where
    I: Iterator<Item = &'a FormalParameter<'a>>,
  {
    let mut seen: HashSet<&str> = HashSet::new();

    for param in params {
      if let BindingPattern::BindingIdentifier(ident) = &param.pattern {
        if !seen.insert(ident.name.as_str()) {
          self.error_ranges.insert(range);
        }
      }
    }
  }
}

impl Handler<'_> for NoDupeArgsHandler {
  fn function(&mut self, function: &Function, _ctx: &mut Context) {
    self.check_params(function.span, function.params.items.iter());
  }

  fn arrow_function_expression(
    &mut self,
    arrow_expr: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    self.check_params(arrow_expr.span, arrow_expr.params.items.iter());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.11.0/tests/lib/rules/no-dupe-args.js
  // MIT Licensed.

  #[test]
  fn no_dupe_args_valid() {
    assert_lint_ok! {
      NoDupeArgs,
      "function a(a, b, c) {}",
      "let a = function (a, b, c) {}",
      "const a = (a, b, c) => {}",
      "function a({a, b}, {c, d}) {}",
      "function a([, a]) {}",
      "function foo([[a, b], [c, d]]) {}",
      "function foo([[a, b], [c, d]]) {}",
      "function foo([[a, b], [c, d]]) {}",
      "const {a, b, c} = obj;",
      "const {a, b, c, a} = obj;",

      // nested
      r#"
function foo(a, b) {
  function bar(b, c) {}
}
    "#,
    };
  }

  #[test]
  fn no_dupe_args_invalid() {
    assert_lint_err! {
      NoDupeArgs,
      "function dupeArgs1(a, b, a) {}": [
        {
          col: 0,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "function a(a, b, b) {}": [
        {
          col: 0,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "function a(a, a, a) {}": [
        {
          col: 0,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "function a(a, b, a) {}": [
        {
          col: 0,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "function a(a, b, a, b)": [
        {
          col: 0,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "let a = function (a, b, b) {}": [
        {
          col: 8,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "let a = function (a, a, a) {}": [
        {
          col: 8,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "let a = function (a, b, a) {}": [
        {
          col: 8,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "let a = function (a, b, a, b) {}": [
        {
          col: 8,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],

      // ESLint's no-dupe-args doesn't check parameters in arrow functions or class methods.
      // cf. https://eslint.org/docs/rules/no-dupe-args
      // But we *do* check them.
      "const dupeArgs = (a, b, a) => {}": [
        {
          col: 17,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "const obj = { foo(a, b, a) {} };": [
        {
          col: 17,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "class Foo { method(a, b, a) {} }": [
        {
          col: 18,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],

      // nested
      r#"
function foo(a, b) {
  function bar(a, b, b) {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      r#"
const foo = (a, b) => {
  const bar = (c, d, d) => {};
};
      "#: [
        {
          line: 3,
          col: 14,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ]
    };
  }
}
