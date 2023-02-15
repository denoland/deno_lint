// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::{view as ast_view, MediaType, SourceRanged};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct ExplicitFunctionReturnType;

const CODE: &str = "explicit-function-return-type";

#[derive(Display)]
enum ExplicitFunctionReturnTypeMessage {
  #[display(fmt = "Missing return type on function")]
  MissingRetType,
}

#[derive(Display)]
enum ExplicitFunctionReturnTypeHint {
  #[display(fmt = "Add a return type to the function signature")]
  AddRetType,
}

impl LintRule for ExplicitFunctionReturnType {
  fn new() -> Arc<Self> {
    Arc::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    // ignore js(x) files
    if matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx) {
      return;
    }
    ExplicitFunctionReturnTypeHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/explicit_function_return_type.md")
  }
}

struct ExplicitFunctionReturnTypeHandler;

impl Handler for ExplicitFunctionReturnTypeHandler {
  fn function(&mut self, function: &ast_view::Function, context: &mut Context) {
    let is_method_setter = matches!(
      function
        .parent()
        .to::<ast_view::ClassMethod>()
        .map(|m| m.method_kind()),
      Some(ast_view::MethodKind::Setter)
    );

    if function.return_type.is_none() && !is_method_setter {
      context.add_diagnostic_with_hint(
        function.range(),
        CODE,
        ExplicitFunctionReturnTypeMessage::MissingRetType,
        ExplicitFunctionReturnTypeHint::AddRetType,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn explicit_function_return_type_valid() {
    assert_lint_ok! {
      ExplicitFunctionReturnType,
      "function fooTyped(): void { }",
      "const bar = (a: string) => { }",
      "const barTyped = (a: string): Promise<void> => { }",
      "class Test { set test(value: string) {} }",
      "const obj = { set test(value: string) {} };",
    };

    assert_lint_ok! {
      ExplicitFunctionReturnType,
      filename: "foo.js",
      "function foo() { }",
      "const bar = (a) => { }",
      "class Test { set test(value) {} }",
      "const obj = { set test(value) {} };",
    };

    assert_lint_ok! {
      ExplicitFunctionReturnType,
      filename: "foo.jsx",
      "export function Foo(props) {return <div>{props.name}</div>}",
      "export default class Foo { render() { return <div></div>}}"
    };
  }

  #[test]
  fn explicit_function_return_type_invalid() {
    assert_lint_err! {
      ExplicitFunctionReturnType,

      r#"function foo() { }"#: [
      {
        col: 0,
        message: ExplicitFunctionReturnTypeMessage::MissingRetType,
        hint: ExplicitFunctionReturnTypeHint::AddRetType,
      }],
      r#"
function a() {
  function b() {}
}
      "#: [
      {
        line: 2,
        col: 0,
        message: ExplicitFunctionReturnTypeMessage::MissingRetType,
        hint: ExplicitFunctionReturnTypeHint::AddRetType,
      },
      {
        line: 3,
        col: 2,
        message: ExplicitFunctionReturnTypeMessage::MissingRetType,
        hint: ExplicitFunctionReturnTypeHint::AddRetType,
      },
      ]
    }
  }
}
