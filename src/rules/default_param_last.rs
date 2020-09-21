// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{ArrowExpr, Function, Pat};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::{self, noop_visit_type};

use std::sync::Arc;

pub struct DefaultParamLast;

impl LintRule for DefaultParamLast {
  fn new() -> Box<Self> {
    Box::new(DefaultParamLast)
  }

  fn code(&self) -> &'static str {
    "default-param-last"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = DefaultParamLastVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct DefaultParamLastVisitor {
  context: Arc<Context>,
}

impl DefaultParamLastVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn report(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "default-param-last",
      "default parameters should be at last",
    );
  }

  fn check_params<'a, 'b, I>(&'a self, params: I)
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

impl Visit for DefaultParamLastVisitor {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, parent: &dyn Node) {
    self.check_params(function.params.iter().rev().map(|p| &p.pat));
    visit::visit_function(self, function, parent);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, parent: &dyn Node) {
    self.check_params(arrow_expr.params.iter().rev());
    visit::visit_arrow_expr(self, arrow_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.9.0/tests/lib/rules/default-param-last.js
  // MIT Licensed.

  #[test]
  fn default_param_last_valid() {
    assert_lint_ok_n::<DefaultParamLast>(vec![
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
    ]);
  }

  #[test]
  fn default_param_last_invalid() {
    assert_lint_err::<DefaultParamLast>("function f(a = 2, b) {}", 11);
    assert_lint_err::<DefaultParamLast>("const f = function (a = 2, b) {}", 20);
    assert_lint_err_n::<DefaultParamLast>(
      "function f(a = 5, b = 6, c) {}",
      vec![18, 11],
    );
    assert_lint_err_n::<DefaultParamLast>(
      "function f(a = 5, b, c = 6, d) {}",
      vec![21, 11],
    );
    assert_lint_err::<DefaultParamLast>("function f(a = 5, b, c = 5) {}", 11);
    assert_lint_err::<DefaultParamLast>("const f = (a = 5, b, ...c) => {}", 11);
    assert_lint_err::<DefaultParamLast>(
      "const f = function f (a, b = 5, c) {}",
      25,
    );
    assert_lint_err::<DefaultParamLast>("const f = (a = 5, { b }) => {}", 11);
    assert_lint_err::<DefaultParamLast>("const f = ({ a } = {}, b) => {}", 11);
    assert_lint_err::<DefaultParamLast>(
      "const f = ({ a, b } = { a: 1, b: 2 }, c) => {}",
      11,
    );
    assert_lint_err::<DefaultParamLast>("const f = ([a] = [], b) => {}", 11);
    assert_lint_err::<DefaultParamLast>(
      "const f = ([a, b] = [1, 2], c) => {}",
      11,
    );
    assert_lint_err_on_line::<DefaultParamLast>(
      r#"
function f() {
  function g(a = 5, b) {}
}
"#,
      3,
      13,
    );
    assert_lint_err_on_line::<DefaultParamLast>(
      r#"
const f = () => {
  function g(a = 5, b) {}
}
"#,
      3,
      13,
    );
    assert_lint_err_on_line::<DefaultParamLast>(
      r#"
function f() {
  const g = (a = 5, b) => {}
}
"#,
      3,
      13,
    );
    assert_lint_err_on_line::<DefaultParamLast>(
      r#"
const f = () => {
  const g = (a = 5, b) => {}
}
"#,
      3,
      13,
    );
  }
}
