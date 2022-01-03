// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_util::StringRepr;
use crate::ProgramRef;
use deno_ast::swc::ast::{
  ArrowExpr, AwaitExpr, BlockStmt, BlockStmtOrExpr, ClassMethod, FnDecl,
  FnExpr, ForOfStmt, MethodProp, PrivateMethod,
};
use deno_ast::swc::common::Spanned;
use deno_ast::swc::visit::{Visit, VisitWith};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
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
  fn new() -> Arc<Self> {
    Arc::new(RequireAwait)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = RequireAwaitVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/require_await.md")
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

impl FunctionInfo {
  fn should_report(self) -> Option<RequireAwaitMessage> {
    if self.is_async && !self.is_generator && !self.is_empty && !self.has_await
    {
      Some(self.kind.into())
    } else {
      None
    }
  }
}

struct RequireAwaitVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  function_info: Option<Box<FunctionInfo>>,
}

impl<'c, 'view> RequireAwaitVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self {
      context,
      function_info: None,
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

    let mut function_info = self.function_info.take().unwrap();

    let upper = function_info.upper.take();

    // Check if the function should be reported
    if let Some(message) = function_info.should_report() {
      self.context.add_diagnostic_with_hint(
        func.span(),
        CODE,
        message,
        RequireAwaitHint::RemoveOrUse,
      );
    }

    // Restore upper function info
    self.function_info = upper;
  }
}

fn is_body_empty(maybe_body: Option<&BlockStmt>) -> bool {
  maybe_body.map_or(true, |body| body.stmts.is_empty())
}

impl<'c, 'view> Visit for RequireAwaitVisitor<'c, 'view> {
  fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Function(Some(
        fn_decl.ident.sym.as_ref().to_string(),
      )),
      is_async: fn_decl.function.is_async,
      is_generator: fn_decl.function.is_generator,
      is_empty: is_body_empty(fn_decl.function.body.as_ref()),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(fn_decl, Box::new(function_info));
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Function(
        fn_expr.ident.as_ref().map(|i| i.sym.as_ref().to_string()),
      ),
      is_async: fn_expr.function.is_async,
      is_generator: fn_expr.function.is_generator,
      is_empty: is_body_empty(fn_expr.function.body.as_ref()),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(fn_expr, Box::new(function_info));
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    let function_info = FunctionInfo {
      kind: FunctionKind::ArrowFunction,
      is_async: arrow_expr.is_async,
      is_generator: arrow_expr.is_generator,
      is_empty: matches!(
        &arrow_expr.body,
        BlockStmtOrExpr::BlockStmt(block_stmt) if block_stmt.stmts.is_empty()
      ),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(arrow_expr, Box::new(function_info));
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(method_prop.key.string_repr()),
      is_async: method_prop.function.is_async,
      is_generator: method_prop.function.is_generator,
      is_empty: is_body_empty(method_prop.function.body.as_ref()),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(method_prop, Box::new(function_info));
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(class_method.key.string_repr()),
      is_async: class_method.function.is_async,
      is_generator: class_method.function.is_generator,
      is_empty: is_body_empty(class_method.function.body.as_ref()),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(class_method, Box::new(function_info));
  }

  fn visit_private_method(&mut self, private_method: &PrivateMethod) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(private_method.key.string_repr()),
      is_async: private_method.function.is_async,
      is_generator: private_method.function.is_generator,
      is_empty: is_body_empty(private_method.function.body.as_ref()),
      upper: self.function_info.take(),
      has_await: false,
    };

    self.process_function(private_method, Box::new(function_info));
  }

  fn visit_await_expr(&mut self, await_expr: &AwaitExpr) {
    if let Some(info) = self.function_info.as_mut() {
      info.has_await = true;
    }

    await_expr.visit_children_with(self);
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt) {
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
      "async function foo() { const a = <number>await doSomething() }",

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
