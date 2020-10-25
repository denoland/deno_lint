// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::{BTreeSet, HashSet};
use swc_common::Span;
use swc_ecmascript::ast::ArrowExpr;
use swc_ecmascript::ast::Function;
use swc_ecmascript::ast::Param;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

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

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoDupeArgsVisitor::new(context);
    program.visit_all_with(program, &mut visitor);
    visitor.report_errors();
  }

  fn docs(&self) -> &'static str {
    r#"Disallows using an argument name more than once in a function signature

If you supply multiple arguments of the same name to a function, the last instance
will shadow the preceding one(s).  This is most likely an unintentional typo.
    
### Invalid:
```typescript
function withDupes(a, b, a) {
  console.log("I'm the value of the second a:", a);
}
```

### Valid:
```typescript
function withoutDupes(a, b, c) {
  console.log("I'm the value of the first (and only) a:", a);
}
```
"#
  }
}

struct NoDupeArgsVisitor<'c> {
  context: &'c mut Context,
  error_spans: BTreeSet<Span>,
}

impl<'c> NoDupeArgsVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self {
      context,
      error_spans: BTreeSet::new(),
    }
  }

  fn report_errors(&mut self) {
    for span in &self.error_spans {
      self.context.add_diagnostic_with_hint(
        *span,
        "no-dupe-args",
        "Duplicate arguments not allowed",
        "Rename or remove the duplicate (e.g. same name) argument",
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
          if !seen.insert(ident.as_ref()) {
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

impl<'c> VisitAll for NoDupeArgsVisitor<'c> {
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
  use crate::test_util::*;

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
      3,
      2,
    );
    assert_lint_err_on_line::<NoDupeArgs>(
      r#"
const foo = (a, b) => {
  const bar = (c, d, d) => {};
};
      "#,
      3,
      14,
    );
  }
}
