// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::HashSet;
use swc_common::Span;
use swc_ecmascript::ast::ArrowExpr;
use swc_ecmascript::ast::Function;
use swc_ecmascript::ast::Param;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoDupeArgs;

impl LintRule for NoDupeArgs {
  fn new() -> Box<Self> {
    Box::new(NoDupeArgs)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-dupe-args"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoDupeArgsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDupeArgsVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDupeArgsVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_pats(&mut self, span: Span, pats: &[Pat]) {
    let mut seen: HashSet<String> = HashSet::new();

    for pat in pats {
      match &pat {
        Pat::Ident(ident) => {
          if !seen.insert(ident.sym.to_string()) {
            self.context.add_diagnostic(
              span,
              "no-dupe-args",
              "Duplicate arguments not allowed",
            );
          }
        }
        _ => continue,
      }
    }
  }

  fn check_params(&mut self, span: Span, params: &[Param]) {
    let pats = params
      .iter()
      .map(|param| param.pat.clone())
      .collect::<Vec<Pat>>();
    self.check_pats(span, &pats);
  }
}

impl<'c> Visit for NoDupeArgsVisitor<'c> {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(function.span, &function.params);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_pats(arrow_expr.span, &arrow_expr.params);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // Some rules are derived from
  // https://github.com/eslint/eslint/blob/v7.11.0/tests/lib/rules/no-dupe-args.js
  // MIT Licensed.

  #[test]
  fn no_dupe_args_valid() {
    assert_lint_ok::<NoDupeArgs>("function a(a, b, c) {}");
    assert_lint_ok::<NoDupeArgs>("let a = function (a, b, c) {}");
    assert_lint_ok::<NoDupeArgs>("function a({a, b}, {c, d}) {}");
    assert_lint_ok::<NoDupeArgs>("function a([, a]) {}");
    assert_lint_ok::<NoDupeArgs>("function foo([[a, b], [c, d]]) {}");
    assert_lint_ok::<NoDupeArgs>("function foo([[a, b], [c, d]]) {}");
    assert_lint_ok::<NoDupeArgs>("function foo([[a, b], [c, d]]) {}");
    assert_lint_ok::<NoDupeArgs>("const {a, b, c} = obj;");
    assert_lint_ok::<NoDupeArgs>("const {a, b, c, a} = obj;");

    // nested
    assert_lint_ok::<NoDupeArgs>(
      r#"
function foo(a, b) {
  function bar(b, c) {}
}
    "#,
    );
  }

  #[test]
  fn no_dupe_args_invalid() {
    assert_lint_err::<NoDupeArgs>("function dupeArgs1(a, b, a) {}", 0);
    // As of Oct 2020, ESLint's no-dupe-args somehow doesn't check parameters in arrow functions,
    // but we *do* check them.
    assert_lint_err::<NoDupeArgs>("const dupeArgs2 = (a, b, a) => {}", 18);

    assert_lint_err::<NoDupeArgs>("function a(a, b, b) {}", 0);
    assert_lint_err::<NoDupeArgs>("function a(a, a, a) {}", 0);
    assert_lint_err::<NoDupeArgs>("function a(a, b, a) {}", 0);
    assert_lint_err::<NoDupeArgs>("function a(a, b, a, b)", 0);
    assert_lint_err::<NoDupeArgs>("let a = function (a, b, b) {}", 8);
    assert_lint_err::<NoDupeArgs>("let a = function (a, a, a) {}", 8);
    assert_lint_err::<NoDupeArgs>("let a = function (a, b, a) {}", 8);
    assert_lint_err::<NoDupeArgs>("let a = function (a, b, a, b) {}", 8);

    // nested
    assert_lint_err_on_line::<NoDupeArgs>(
      r#"
function foo(a, b) {
  function bar(a, b, b) {}
}
      "#,
      2,
      2,
    );
  }
}
