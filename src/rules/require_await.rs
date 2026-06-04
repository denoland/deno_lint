// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, AwaitExpression, ForOfStatement, Function,
  FunctionType, MethodDefinition, MethodDefinitionKind, ObjectProperty,
  Program, PropertyKey, VariableDeclaration, VariableDeclarationKind,
};
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct RequireAwait;

const CODE: &str = "require-await";

#[derive(Display)]
enum RequireAwaitMessage {
  #[display(
    fmt = "Async function '{}' has no 'await' expression or 'await using' declaration.",
    _0
  )]
  Function(String),
  #[display(
    fmt = "Async function has no 'await' expression or 'await using' declaration."
  )]
  AnonymousFunction,
  #[display(
    fmt = "Async arrow function has no 'await' expression or 'await using' declaration."
  )]
  ArrowFunction,
  #[display(
    fmt = "Async method '{}' has no 'await' expression or 'await using' declaration.",
    _0
  )]
  Method(String),
  #[display(
    fmt = "Async method has no 'await' expression or 'await using' declaration."
  )]
  AnonymousMethod,
}

#[derive(Display)]
enum RequireAwaitHint {
  #[display(
    fmt = "Remove 'async' keyword from the function or use 'await' expression or 'await using' declaration inside."
  )]
  RemoveOrUse,
}

impl LintRule for RequireAwait {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = RequireAwaitHandler {
      scope_stack: vec![],
      pending_method_name: None,
      pending_object_method_name: None,
    };
    crate::handler::traverse_program(&mut handler, program, context);
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

struct FunctionScope {
  kind: FunctionKind,
  is_async: bool,
  is_generator: bool,
  is_empty: bool,
  has_await: bool,
  range: Span,
}

fn find_async_keyword_span(span: Span, ctx: &Context) -> Span {
  let source = ctx.source_text();
  let start = span.start as usize;
  let end = std::cmp::min(span.end as usize, source.len());
  let text = &source[start..end];
  if let Some(pos) = text.find("async") {
    Span::new((start + pos) as u32, (start + pos + 5) as u32)
  } else {
    span
  }
}

fn property_key_name(key: &PropertyKey) -> Option<String> {
  match key {
    PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
    PropertyKey::PrivateIdentifier(ident) => Some(format!("#{}", ident.name)),
    _ => {
      if let PropertyKey::StringLiteral(s) = key {
        Some(s.value.to_string())
      } else if let PropertyKey::NumericLiteral(n) = key {
        Some(n.value.to_string())
      } else {
        None
      }
    }
  }
}

struct RequireAwaitHandler {
  scope_stack: Vec<FunctionScope>,
  /// Set by method_definition handler, consumed by function handler
  pending_method_name: Option<Option<String>>,
  /// Set by object_property handler for method shorthand, consumed by function handler
  pending_object_method_name: Option<Option<String>>,
}

impl Handler<'_> for RequireAwaitHandler {
  fn method_definition(
    &mut self,
    method: &MethodDefinition,
    _ctx: &mut Context,
  ) {
    if method.kind == MethodDefinitionKind::Constructor
      || method.kind == MethodDefinitionKind::Get
      || method.kind == MethodDefinitionKind::Set
    {
      // For getters/setters/constructors, don't set pending method name
      // so the inner function is treated as a regular function
      return;
    }
    self.pending_method_name = Some(property_key_name(&method.key));
  }

  fn object_property(&mut self, prop: &ObjectProperty, _ctx: &mut Context) {
    // method shorthand: `{ async foo() {} }`
    if prop.method {
      self.pending_object_method_name = Some(property_key_name(&prop.key));
    }
  }

  fn function(&mut self, function: &Function, ctx: &mut Context) {
    match function.r#type {
      FunctionType::FunctionDeclaration | FunctionType::FunctionExpression => {}
      _ => return,
    }

    // Determine function kind
    let kind = if let Some(method_name) = self.pending_method_name.take() {
      FunctionKind::Method(method_name)
    } else if let Some(method_name) = self.pending_object_method_name.take() {
      FunctionKind::Method(method_name)
    } else {
      let name = function.id.as_ref().map(|id| id.name.to_string());
      FunctionKind::Function(name)
    };

    let range = if function.r#async {
      find_async_keyword_span(function.span, ctx)
    } else {
      match &kind {
        FunctionKind::Function(Some(_)) => function
          .id
          .as_ref()
          .map(|id| id.span)
          .unwrap_or(function.span),
        _ => function.span,
      }
    };

    let is_empty = function
      .body
      .as_ref()
      .is_none_or(|body| body.statements.is_empty());

    self.scope_stack.push(FunctionScope {
      kind,
      is_async: function.r#async,
      is_generator: function.generator,
      is_empty,
      has_await: false,
      range,
    });
  }

  fn function_exit(&mut self, function: &Function, ctx: &mut Context) {
    match function.r#type {
      FunctionType::FunctionDeclaration | FunctionType::FunctionExpression => {}
      _ => return,
    }
    if let Some(scope) = self.scope_stack.pop() {
      report_if_needed(scope, ctx);
    }
  }

  fn arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression,
    ctx: &mut Context,
  ) {
    let is_empty = arrow.body.statements.is_empty() && !arrow.expression;
    let range = if arrow.r#async {
      find_async_keyword_span(arrow.span, ctx)
    } else {
      arrow.span
    };
    self.scope_stack.push(FunctionScope {
      kind: FunctionKind::ArrowFunction,
      is_async: arrow.r#async,
      is_generator: false,
      is_empty,
      has_await: false,
      range,
    });
  }

  fn arrow_function_expression_exit(
    &mut self,
    _arrow: &ArrowFunctionExpression,
    ctx: &mut Context,
  ) {
    if let Some(scope) = self.scope_stack.pop() {
      report_if_needed(scope, ctx);
    }
  }

  fn await_expression(&mut self, _n: &AwaitExpression, _ctx: &mut Context) {
    if let Some(scope) = self.scope_stack.last_mut() {
      scope.has_await = true;
    }
  }

  fn for_of_statement(&mut self, for_of: &ForOfStatement, _ctx: &mut Context) {
    if for_of.r#await {
      if let Some(scope) = self.scope_stack.last_mut() {
        scope.has_await = true;
      }
    }
  }

  fn variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration,
    _ctx: &mut Context,
  ) {
    if var_decl.kind == VariableDeclarationKind::AwaitUsing {
      if let Some(scope) = self.scope_stack.last_mut() {
        scope.has_await = true;
      }
    }
  }
}

fn report_if_needed(scope: FunctionScope, ctx: &mut Context) {
  if scope.is_async
    && !scope.is_generator
    && !scope.is_empty
    && !scope.has_await
  {
    let message: RequireAwaitMessage = scope.kind.into();
    ctx.add_diagnostic_with_hint(
      scope.range,
      CODE,
      message,
      RequireAwaitHint::RemoveOrUse,
    );
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
      "async function foo() { const bar = <number>await doSomething() }",

      // empty functions are ok.
      "async function foo() {}",
      "async () => {}",

      // normal functions are ok.
      "function foo() { doSomething() }",

      // for-await-of
      "async function foo() { for await (x of xs); }",

      // using-await
      "async function foo() { await using foo = doSomething(); }",

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
          col: 12,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "class A { async foo() { doSomething() } }": [
        {
          col: 19,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "class A { private async foo() { doSomething() } }": [
        {
          col: 27,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "(class { async foo() { doSomething() } })": [
        {
          col: 18,
          message: variant!(RequireAwaitMessage, Method, "foo"),
          hint: RequireAwaitHint::RemoveOrUse,
        },
      ],
      "(class { async ''() { doSomething() } })": [
        {
          col: 17,
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
