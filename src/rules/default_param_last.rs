// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{ArrowExpr, Function, Pat};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

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
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = DefaultParamLastVisitor::new(context);
    module.visit_all_with(module, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Enforces default parameter(s) to be last in the function signature.

Parameters with default values are optional by nature but cannot be left out
of the function call without mapping the function inputs to different parameters
which is confusing and error prone.  Specifying them last allows them to be left
out without changing the semantics of the other parameters.
    
### Valid:
```typescript
function f() {}
function f(a) {}
function f(a = 5) {}
function f(a, b = 5) {}
function f(a, b = 5, c = 5) {}
function f(a, b = 5, ...c) {}
function f(a = 2, b = 3) {}
```

### Invalid:
```typescript
function f(a = 2, b) {}
function f(a = 5, b, c = 5) {}
```"#
  }
}

struct DefaultParamLastVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> DefaultParamLastVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn report(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      "default-param-last",
      "default parameters should be at last",
      "Modify the signatures to move default parameter(s) to the end",
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

impl<'c> VisitAll for DefaultParamLastVisitor<'c> {
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
  use crate::test_util::*;

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
class Foo {
  bar(a = 2, b) {}
}
      "#,
      3,
      6,
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
    assert_lint_err_on_line::<DefaultParamLast>(
      r#"
class Foo {
  bar(a, b = 1) {
    class X {
      y(c = 3, d) {}
    }
  }
}
      "#,
      5,
      8,
    );
  }
}
