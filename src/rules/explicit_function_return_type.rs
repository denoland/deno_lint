// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;
use derive_more::Display;

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
  fn new() -> Box<Self> {
    Box::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = ExplicitFunctionReturnTypeVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/explicit_function_return_type.md")
  }
}

struct ExplicitFunctionReturnTypeVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> ExplicitFunctionReturnTypeVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for ExplicitFunctionReturnTypeVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_function(
    &mut self,
    function: &deno_ast::swc::ast::Function,
    _parent: &dyn Node,
  ) {
    if function.return_type.is_none() {
      self.context.add_diagnostic_with_hint(
        function.span,
        CODE,
        ExplicitFunctionReturnTypeMessage::MissingRetType,
        ExplicitFunctionReturnTypeHint::AddRetType,
      );
    }
    for stmt in &function.body {
      self.visit_block_stmt(stmt, _parent);
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
