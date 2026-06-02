// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{
  Function, MethodDefinition, MethodDefinitionKind, ObjectProperty, Program,
  PropertyKind,
};
use deno_ast::MediaType;
use derive_more::Display;

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
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    // ignore js(x) files
    if matches!(context.media_type(), MediaType::JavaScript | MediaType::Jsx) {
      return;
    }
    let mut handler = ExplicitFunctionReturnTypeHandler {
      skip_next_function: false,
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct ExplicitFunctionReturnTypeHandler {
  /// When true, the next `function` callback should be skipped (it's a setter's function).
  skip_next_function: bool,
}

impl Handler<'_> for ExplicitFunctionReturnTypeHandler {
  fn method_definition(
    &mut self,
    method_def: &MethodDefinition,
    _ctx: &mut Context,
  ) {
    if method_def.kind == MethodDefinitionKind::Set {
      self.skip_next_function = true;
    }
  }

  fn object_property(&mut self, prop: &ObjectProperty, _ctx: &mut Context) {
    if prop.kind == PropertyKind::Set {
      self.skip_next_function = true;
    }
  }

  fn function(&mut self, function: &Function, ctx: &mut Context) {
    if self.skip_next_function {
      self.skip_next_function = false;
      return;
    }

    if function.return_type.is_none() {
      ctx.add_diagnostic_with_hint(
        function.span,
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
      filename: "file:///foo.js",
      "function foo() { }",
      "const bar = (a) => { }",
      "class Test { set test(value) {} }",
      "const obj = { set test(value) {} };",
    };

    assert_lint_ok! {
      ExplicitFunctionReturnType,
      filename: "file:///foo.jsx",
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
