// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::ArrowExpr;
use deno_ast::swc::ast::Function;
use deno_ast::swc::ast::Param;
use deno_ast::swc::ast::Pat;
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::{VisitAll, VisitAllWith};
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
  fn new() -> Box<Self> {
    Box::new(NoDupeArgs)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-dupe-args"
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoDupeArgsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
    visitor.report_errors();
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_dupe_args.md")
  }
}

struct NoDupeArgsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  error_spans: BTreeSet<Span>,
}

impl<'c, 'view> NoDupeArgsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self {
      context,
      error_spans: BTreeSet::new(),
    }
  }

  fn report_errors(&mut self) {
    for span in &self.error_spans {
      self.context.add_diagnostic_with_hint(
        *span,
        CODE,
        NoDupeArgsMessage::Unexpected,
        NoDupeArgsHint::RenameOrRemove,
      );
    }
  }

  fn check_pats<'a, 'b, I>(&'a mut self, span: Span, pats: I)
  where
    I: Iterator<Item = &'b Pat>,
  {
    let mut seen: HashSet<&str> = HashSet::new();

    for pat in pats {
      match &pat {
        Pat::Ident(ident) => {
          if !seen.insert(ident.id.as_ref()) {
            self.error_spans.insert(span);
          }
        }
        _ => continue,
      }
    }
  }

  fn check_params<'a, 'b, I>(&'a mut self, span: Span, params: I)
  where
    I: Iterator<Item = &'b Param>,
  {
    let pats = params.map(|param| &param.pat);
    self.check_pats(span, pats);
  }
}

impl<'ctx, 'view> VisitAll for NoDupeArgsVisitor<'ctx, 'view> {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(function.span, function.params.iter());
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_pats(arrow_expr.span, arrow_expr.params.iter());
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
          col: 14,
          message: NoDupeArgsMessage::Unexpected,
          hint: NoDupeArgsHint::RenameOrRemove,
        }
      ],
      "class Foo { method(a, b, a) {} }": [
        {
          col: 12,
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
