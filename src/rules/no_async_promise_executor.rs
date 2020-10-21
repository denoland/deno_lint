// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Expr, NewExpr, ParenExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct NoAsyncPromiseExecutor;

impl LintRule for NoAsyncPromiseExecutor {
  fn new() -> Box<Self> {
    Box::new(NoAsyncPromiseExecutor)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-async-promise-executor"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoAsyncPromiseExecutorVisitor::new(context);
    module.visit_all_with(module, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Requires that async promise executor functions are not used

Promise constructors take an executor function as an argument with `resolve` and 
`reject` parameters that can be used to control the state of the created Promise.
This function is allowed to be async but this is generally not a good idea for
several reasons:
* If an async executor function throws an error, the error will be lost and won't
cause the newly-constructed Promise to reject. This could make it difficult to
debug and handle some errors.
* If an async Promise executor function is using await, then this is usually a
sign that it is not actually necessary to use the new Promise constructor and the
code can be restructured to avoid the use of a promise, or the scope of the new
Promise constructor can be reduced, extracting the async code and changing it to
be synchronous.

### Invalid:
```typescript
new Promise(async function(resolve, reject) {});
new Promise(async (resolve, reject) => {});
```
    
### Valid:
```typescript
new Promise(function(resolve, reject) {});
new Promise((resolve, reject) => {});
```
"#
  }
}

struct NoAsyncPromiseExecutorVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoAsyncPromiseExecutorVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

fn is_async_function(expr: &Expr) -> bool {
  match expr {
    Expr::Fn(fn_expr) => fn_expr.function.is_async,
    Expr::Arrow(arrow_expr) => arrow_expr.is_async,
    Expr::Paren(ParenExpr { ref expr, .. }) => is_async_function(&**expr),
    _ => false,
  }
}

impl<'c> VisitAll for NoAsyncPromiseExecutorVisitor<'c> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.as_ref();
      if name != "Promise" {
        return;
      }

      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          if is_async_function(&*first_arg.expr) {
            self.context.add_diagnostic_with_hint(
              new_expr.span,
              "no-async-promise-executor",
              "Async promise executors are not allowed",
              "Remove `async` from executor function and adjust promise code as needed",
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_async_promise_executor_valid() {
    assert_lint_ok! {
      NoAsyncPromiseExecutor,
      "new Promise(function(resolve, reject) {});",
      "new Promise((resolve, reject) => {});",
      "new Promise((resolve, reject) => {}, async function unrelated() {})",
      "new Foo(async (resolve, reject) => {})",
      "new class { foo() { new Promise(function(resolve, reject) {}); } }",
    };
  }

  #[test]
  fn no_async_promise_executor_invalid() {
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async function(resolve, reject) {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async function foo(resolve, reject) {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(async (resolve, reject) => {});",
      0,
    );
    assert_lint_err::<NoAsyncPromiseExecutor>(
      "new Promise(((((async () => {})))));",
      0,
    );
    // nested
    assert_lint_err_on_line::<NoAsyncPromiseExecutor>(
      r#"
const a = new class {
  foo() {
    let b = new Promise(async function(resolve, reject) {});
  }
}
      "#,
      4,
      12,
    );
  }
}
