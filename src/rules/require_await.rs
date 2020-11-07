// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::StringRepr;
use derive_more::Display;
use std::mem;
use swc_common::Spanned;
use swc_ecmascript::ast::{
  ArrowExpr, AwaitExpr, BlockStmtOrExpr, ClassMethod, FnDecl, FnExpr,
  ForOfStmt, MethodProp, PrivateMethod,
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{Visit, VisitWith};

pub struct RequireAwait;

const CODE: &str = "require-await";

#[derive(Display)]
enum RequireAwaitMessage {
  #[display(fmt = "Async function '{}' has no 'await' expression.", _0)]
  Function(String),
  #[display(fmt = "Async function has no 'await' expression.")]
  AnonymousFunction,
  #[display(fmt = "Async arrow function has no 'await' expression.")]
  ArrowFunction,
  #[display(fmt = "Async method '{}' has no 'await' expression.", _0)]
  Method(String),
  #[display(fmt = "Async method has no 'await' expression.")]
  AnonymousMethod,
}

#[derive(Display)]
enum RequireAwaitHint {
  #[display(
    fmt = "Remove 'async' keyword from the function or use 'await' expression inside."
  )]
  RemoveOrUse,
}

impl LintRule for RequireAwait {
  fn new() -> Box<Self> {
    Box::new(RequireAwait)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = RequireAwaitVisitor::new(context);
    program.visit_with(program, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows async functions that have no await expression

In general, the primary reason to use async functions is to use await expressions inside.
If an async function has no await expression, it is most likely an unintentional mistake.

### Invalid:
```typescript
async function f1() {
  doSomething();
}

const f2 = async () => {
  doSomething();
};

const f3 = async () => doSomething();

const obj = {
  async method() {
    doSomething();
  }
};

class MyClass {
  async method() {
    doSomething();
  }
}
```

### Valid:
```typescript
await asyncFunction();

function normalFunction() {
  doSomething();
}

async function f1() {
  await asyncFunction();
}

const f2 = async () => {
  await asyncFunction();
};

const f3 = async () => await asyncFunction();

async function f4() {
  for await (const num of asyncIterable) {
    console.log(num);
  }
}

// empty functions are valid
async function emptyFunction() {}
const emptyArrowFunction = async () => {};

// generators are also valid
async function* gen() {
  console.log(42);
}
```
"#
  }
}

enum FunctionKind {
  Function(Option<String>),
  ArrowFunction,
  Method(Option<String>),
}

impl From<FunctionKind> for RequireAwaitMessage {
  fn from(kind: FunctionKind) -> Self {
    match kind {
      FunctionKind::Function(Some(name)) => RequireAwaitMessage::Function(name),
      FunctionKind::Function(None) => RequireAwaitMessage::AnonymousFunction,
      FunctionKind::ArrowFunction => RequireAwaitMessage::ArrowFunction,
      FunctionKind::Method(Some(name)) => RequireAwaitMessage::Method(name),
      FunctionKind::Method(None) => RequireAwaitMessage::AnonymousMethod,
    }
  }
}

impl Default for FunctionKind {
  fn default() -> Self {
    FunctionKind::Function(None)
  }
}

struct FunctionInfo {
  kind: FunctionKind,
  is_async: bool,
  is_generator: bool,
  is_empty: bool,
  has_await: bool,
  upper: Option<Box<FunctionInfo>>,
}

#[derive(Default)]
struct FunctionInfoBuilder {
  kind: Option<FunctionKind>,
  is_async: Option<bool>,
  is_generator: Option<bool>,
  is_empty: Option<bool>,
  upper: Option<Box<FunctionInfo>>,
}

impl FunctionInfo {
  fn builder() -> FunctionInfoBuilder {
    FunctionInfoBuilder::default()
  }

  fn should_report(&mut self) -> Option<RequireAwaitMessage> {
    if self.is_async && !self.is_generator && !self.is_empty && !self.has_await
    {
      let kind = mem::take(&mut self.kind);
      Some(kind.into())
    } else {
      None
    }
  }
}

impl FunctionInfoBuilder {
  fn kind(mut self, kind: FunctionKind) -> Self {
    self.kind = Some(kind);
    self
  }

  #[allow(clippy::wrong_self_convention)]
  fn is_async(mut self, is_async: bool) -> Self {
    self.is_async = Some(is_async);
    self
  }

  #[allow(clippy::wrong_self_convention)]
  fn is_generator(mut self, is_generator: bool) -> Self {
    self.is_generator = Some(is_generator);
    self
  }

  #[allow(clippy::wrong_self_convention)]
  fn is_empty(mut self, is_empty: bool) -> Self {
    self.is_empty = Some(is_empty);
    self
  }

  fn upper(mut self, upper: Option<Box<FunctionInfo>>) -> Self {
    self.upper = upper;
    self
  }

  fn build(self) -> Box<FunctionInfo> {
    Box::new(FunctionInfo {
      kind: self.kind.unwrap_or_default(),
      is_async: self.is_async.unwrap_or_default(),
      is_generator: self.is_generator.unwrap_or_default(),
      is_empty: self.is_empty.unwrap_or_default(),
      has_await: false,
      upper: self.upper,
    })
  }
}

struct RequireAwaitVisitor<'c> {
  context: &'c mut Context,
  function_info: Option<Box<FunctionInfo>>,
}

impl<'c> RequireAwaitVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self {
      context,
      function_info: None,
    }
  }

  fn check_function_info(&mut self, span: impl Spanned) {
    if let Some(message) = self.function_info.as_mut().unwrap().should_report()
    {
      self.context.add_diagnostic_with_hint(
        span.span(),
        CODE,
        message,
        RequireAwaitHint::RemoveOrUse,
      );
    }
  }

  fn process_function<F>(
    &mut self,
    func: &F,
    new_function_info: Box<FunctionInfo>,
  ) where
    F: VisitWith<Self> + Spanned,
  {
    // Set the current function's info
    self.function_info = Some(new_function_info);

    // Visit the function's inside
    func.visit_children_with(self);

    // Check if the function should be reported
    self.check_function_info(func);

    // Restore upper function info
    let upper = mem::take(&mut self.function_info.as_mut().unwrap().upper);
    self.function_info = upper;
  }
}

impl<'c> Visit for RequireAwaitVisitor<'c> {
  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::Function(Some(
        fn_decl.ident.sym.as_ref().to_string(),
      )))
      .is_async(fn_decl.function.is_async)
      .is_generator(fn_decl.function.is_generator)
      .is_empty(
        fn_decl
          .function
          .body
          .as_ref()
          .map_or(true, |body| body.stmts.is_empty()),
      )
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(fn_decl, function_info);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, _: &dyn Node) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::Function(
        fn_expr.ident.as_ref().map(|i| i.sym.as_ref().to_string()),
      ))
      .is_async(fn_expr.function.is_async)
      .is_generator(fn_expr.function.is_generator)
      .is_empty(
        fn_expr
          .function
          .body
          .as_ref()
          .map_or(true, |body| body.stmts.is_empty()),
      )
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(fn_expr, function_info);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::ArrowFunction)
      .is_async(arrow_expr.is_async)
      .is_generator(arrow_expr.is_generator)
      .is_empty(matches!(
      &arrow_expr.body,
      BlockStmtOrExpr::BlockStmt(block_stmt) if block_stmt.stmts.is_empty()
      ))
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(arrow_expr, function_info);
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp, _: &dyn Node) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::Method(method_prop.key.string_repr()))
      .is_async(method_prop.function.is_async)
      .is_generator(method_prop.function.is_generator)
      .is_empty(
        method_prop
          .function
          .body
          .as_ref()
          .map_or(true, |body| body.stmts.is_empty()),
      )
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(method_prop, function_info);
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod, _: &dyn Node) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::Method(class_method.key.string_repr()))
      .is_async(class_method.function.is_async)
      .is_generator(class_method.function.is_generator)
      .is_empty(
        class_method
          .function
          .body
          .as_ref()
          .map_or(true, |body| body.stmts.is_empty()),
      )
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(class_method, function_info);
  }

  fn visit_private_method(
    &mut self,
    private_method: &PrivateMethod,
    _: &dyn Node,
  ) {
    let function_info = FunctionInfo::builder()
      .kind(FunctionKind::Method(private_method.key.string_repr()))
      .is_async(private_method.function.is_async)
      .is_generator(private_method.function.is_generator)
      .is_empty(
        private_method
          .function
          .body
          .as_ref()
          .map_or(true, |body| body.stmts.is_empty()),
      )
      .upper(mem::take(&mut self.function_info))
      .build();

    self.process_function(private_method, function_info);
  }

  fn visit_await_expr(&mut self, await_expr: &AwaitExpr, _: &dyn Node) {
    if let Some(info) = self.function_info.as_mut() {
      info.has_await = true;
    }

    await_expr.visit_children_with(self);
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _: &dyn Node) {
    if for_of_stmt.await_token.is_some() {
      if let Some(info) = self.function_info.as_mut() {
        info.has_await = true;
      }
    }

    for_of_stmt.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.13.0/tests/lib/rules/require-await.js
  // MIT Licensed.

  #[test]
  fn require_await_valid() {
    assert_lint_ok! {
      RequireAwait,
      "async function foo() { await doSomething() }",
      "(async function() { await doSomething() })",
      "async () => { await doSomething() }",
      "async () => await doSomething()",
      "({ async foo() { await doSomething() } })",
      "class A { async foo() { await doSomething() } }",
      "(class { async foo() { await doSomething() } })",
      "async function foo() { await (async () => { await doSomething() }) }",

      // empty functions are ok.
      "async function foo() {}",
      "async () => {}",

      // normal functions are ok.
      "function foo() { doSomething() }",

      // for-await-of
      "async function foo() { for await (x of xs); }",

      // global await
      "await foo()",
      r#"
for await (let num of asyncIterable) {
    console.log(num);
}
      "#,

      // generator
      "async function* run() { yield * anotherAsyncGenerator() }",
      r#"
async function* run() {
  await new Promise(resolve => setTimeout(resolve, 100));
  yield 'Hello';
  console.log('World');
}
      "#,
      "async function* run() { }",
      "const foo = async function *(){}",
      r#"const foo = async function *(){ console.log("bar") }"#,
      r#"async function* run() { console.log("bar") }"#,
    };
  }

  #[test]
  fn require_await_invalid() {
    assert_lint_err! {
      RequireAwait,
      "async function foo() { doSomething() }": [
        {
          col: 0,
          message: variant!(RequireAwaitMessage, Function, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "(async function() { doSomething() })": [
        {
          col: 1,
          message: RequireAwaitMessage::AnonymousFunction,
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "async () => { doSomething() }": [
        {
          col: 0,
          message: RequireAwaitMessage::ArrowFunction,
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "async () => doSomething()": [
        {
          col: 0,
          message: RequireAwaitMessage::ArrowFunction,
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "({ async foo() { doSomething() } })": [
        {
          col: 3,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "class A { async foo() { doSomething() } }": [
        {
          col: 10,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "class A { private async foo() { doSomething() } }": [
        {
          col: 10,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "(class { async foo() { doSomething() } })": [
        {
          col: 9,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "(class { async ''() { doSomething() } })": [
        {
          col: 9,
          message: variant!(RequireAwaitMessage, Method, ""),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "async function foo() { async () => { await doSomething() } }": [
        {
          col: 0,
          message: variant!(RequireAwaitMessage, Function, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "async function foo() { await (async () => { doSomething() }) }": [
        {
          col: 30,
          message: RequireAwaitMessage::ArrowFunction,
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
    };
  }
}
