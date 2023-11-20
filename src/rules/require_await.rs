// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;

use deno_ast::swc::parser::token::{Token, Word};
use deno_ast::view::{NodeTrait, Program};
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use derive_more::Display;

#[derive(Debug)]
pub struct RequireAwait;

const CODE: &str = "require-await";

#[derive(Display)]
enum RequireAwaitMessage {
  #[display(
    fmt = "Async function '{}' has no 'await' expression or 'using await' declaration.",
    _0
  )]
  Function(String),
  #[display(
    fmt = "Async function has no 'await' expression or 'using await' declaration."
  )]
  AnonymousFunction,
  #[display(
    fmt = "Async arrow function has no 'await' expression or 'using await' declaration."
  )]
  ArrowFunction,
  #[display(
    fmt = "Async method '{}' has no 'await' expression or 'using await' declaration.",
    _0
  )]
  Method(String),
  #[display(
    fmt = "Async method has no 'await' expression or 'using await' declaration."
  )]
  AnonymousMethod,
}

#[derive(Display)]
enum RequireAwaitHint {
  #[display(
    fmt = "Remove 'async' keyword from the function or use 'await' expression or 'using await' declaration inside."
  )]
  RemoveOrUse,
}

impl LintRule for RequireAwait {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    RequireAwaitHandler.traverse(program, context);
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

fn find_async_token_range(
  node: deno_ast::view::Node,
  ctx: &Context,
) -> SourceRange {
  node
    .tokens_fast(ctx.program())
    .iter()
    .find(|t| t.token == Token::Word(Word::Ident("async".into())))
    .expect("there must be a async span")
    .range()
}

struct RequireAwaitHandler;

impl Handler for RequireAwaitHandler {
  fn fn_decl(&mut self, fn_decl: &deno_ast::view::FnDecl, ctx: &mut Context) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Function(Some(
        fn_decl.ident.sym().as_ref().to_string(),
      )),
      is_async: fn_decl.function.is_async(),
      is_generator: fn_decl.function.is_generator(),
      is_empty: is_body_empty(fn_decl.function.body),
      has_await: false,
    };

    let range = if function_info.is_async {
      find_async_token_range(fn_decl.as_node(), ctx)
    } else {
      fn_decl.ident.range()
    };

    process_function(fn_decl.as_node(), range, function_info, ctx);
  }

  fn fn_expr(&mut self, fn_expr: &deno_ast::view::FnExpr, ctx: &mut Context) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Function(
        fn_expr.ident.as_ref().map(|i| i.sym().as_ref().to_string()),
      ),
      is_async: fn_expr.function.is_async(),
      is_generator: fn_expr.function.is_generator(),
      is_empty: is_body_empty(fn_expr.function.body),
      has_await: false,
    };
    let range = if function_info.is_async {
      find_async_token_range(fn_expr.as_node(), ctx)
    } else {
      fn_expr.range()
    };
    process_function(fn_expr.as_node(), range, function_info, ctx);
  }

  fn arrow_expr(
    &mut self,
    arrow_expr: &deno_ast::view::ArrowExpr,
    ctx: &mut Context,
  ) {
    let function_info = FunctionInfo {
      kind: FunctionKind::ArrowFunction,
      is_async: arrow_expr.is_async(),
      is_generator: arrow_expr.is_generator(),
      is_empty: matches!(
        &arrow_expr.body,
        deno_ast::view::BlockStmtOrExpr::BlockStmt(block_stmt) if block_stmt.stmts.is_empty()
      ),
      has_await: false,
    };
    let range = if function_info.is_async {
      find_async_token_range(arrow_expr.as_node(), ctx)
    } else {
      arrow_expr.range()
    };
    process_function(arrow_expr.as_node(), range, function_info, ctx);
  }

  fn method_prop(
    &mut self,
    method_prop: &deno_ast::view::MethodProp,
    ctx: &mut Context,
  ) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(method_prop.key.string_repr()),
      is_async: method_prop.function.is_async(),
      is_generator: method_prop.function.is_generator(),
      is_empty: is_body_empty(method_prop.function.body),
      has_await: false,
    };

    let range = if function_info.is_async {
      find_async_token_range(method_prop.as_node(), ctx)
    } else {
      method_prop.inner.key.range()
    };

    process_function(method_prop.as_node(), range, function_info, ctx);
  }

  fn class_method(
    &mut self,
    class_method: &deno_ast::view::ClassMethod,
    ctx: &mut Context,
  ) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(class_method.key.string_repr()),
      is_async: class_method.function.is_async(),
      is_generator: class_method.function.is_generator(),
      is_empty: is_body_empty(class_method.function.body),
      has_await: false,
    };

    let range = if function_info.is_async {
      find_async_token_range(class_method.as_node(), ctx)
    } else {
      class_method.inner.key.range()
    };

    process_function(class_method.as_node(), range, function_info, ctx);
  }

  fn private_method(
    &mut self,
    private_method: &deno_ast::view::PrivateMethod,
    ctx: &mut Context,
  ) {
    let function_info = FunctionInfo {
      kind: FunctionKind::Method(private_method.key.string_repr()),
      is_async: private_method.function.is_async(),
      is_generator: private_method.function.is_generator(),
      is_empty: is_body_empty(private_method.function.body),
      has_await: false,
    };
    let range = if function_info.is_async {
      find_async_token_range(private_method.as_node(), ctx)
    } else {
      private_method.inner.key.range()
    };
    process_function(private_method.as_node(), range, function_info, ctx);
  }
}

struct FunctionHandler {
  function_info: Option<Box<FunctionInfo>>,
}

impl Handler for FunctionHandler {
  fn fn_decl(&mut self, _n: &deno_ast::view::FnDecl, ctx: &mut Context) {
    ctx.stop_traverse();
  }

  fn fn_expr(&mut self, _n: &deno_ast::view::FnExpr, ctx: &mut Context) {
    ctx.stop_traverse();
  }

  fn arrow_expr(&mut self, _n: &deno_ast::view::ArrowExpr, ctx: &mut Context) {
    ctx.stop_traverse();
  }

  fn method_prop(
    &mut self,
    _n: &deno_ast::view::MethodProp,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn class_method(
    &mut self,
    _n: &deno_ast::view::ClassMethod,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn private_method(
    &mut self,
    _n: &deno_ast::view::PrivateMethod,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn await_expr(&mut self, _n: &deno_ast::view::AwaitExpr, _ctx: &mut Context) {
    if let Some(info) = self.function_info.as_mut() {
      info.has_await = true;
    }
  }

  fn for_of_stmt(
    &mut self,
    for_of_stmt: &deno_ast::view::ForOfStmt,
    ctx: &mut Context,
  ) {
    for_of_stmt.tokens_fast(ctx.program());
    if for_of_stmt.is_await() {
      if let Some(info) = self.function_info.as_mut() {
        info.has_await = true;
      }
    }
  }

  fn using_decl(
    &mut self,
    using_decl: &deno_ast::view::UsingDecl,
    _ctx: &mut Context,
  ) {
    if using_decl.is_await() {
      if let Some(info) = self.function_info.as_mut() {
        info.has_await = true;
      }
    }
  }
}

fn process_function<'a, N>(
  node: N,
  range: SourceRange,
  function_info: FunctionInfo,
  ctx: &mut Context,
) where
  N: NodeTrait<'a>,
{
  let mut function_handler = FunctionHandler {
    function_info: Some(Box::new(function_info)),
  };

  for child in node.children() {
    function_handler.traverse(child, ctx)
  }

  let function_info = function_handler.function_info.take().unwrap();

  if let Some(message) = function_info.should_report() {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      message,
      RequireAwaitHint::RemoveOrUse,
    );
  }
}

fn is_body_empty(maybe_body: Option<&deno_ast::view::BlockStmt>) -> bool {
  maybe_body.map_or(true, |body| body.stmts.is_empty())
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
      "async function foo() { const bar = <number>await doSomething() }",

      // empty functions are ok.
      "async function foo() {}",
      "async () => {}",

      // normal functions are ok.
      "function foo() { doSomething() }",

      // for-await-of
      "async function foo() { for await (x of xs); }",

      // using-await
      "async function foo() { using await = doSomething(); }",

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
          col: 18,
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
