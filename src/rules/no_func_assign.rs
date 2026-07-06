// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrayAssignmentTarget, AssignmentExpression, AssignmentTarget,
  AssignmentTargetMaybeDefault, AssignmentTargetProperty,
  ObjectAssignmentTarget, Program,
};
use deno_ast::oxc::span::Span;
use deno_ast::BindingKind;
use derive_more::Display;

#[derive(Debug)]
pub struct NoFuncAssign;

const CODE: &str = "no-func-assign";

#[derive(Display)]
enum NoFuncAssignMessage {
  #[display(fmt = "Reassigning function declaration is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoFuncAssignHint {
  #[display(
    fmt = "Remove or rework the reassignment of the existing function"
  )]
  RemoveOrRework,
}

impl LintRule for NoFuncAssign {
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
    let mut handler = NoFuncAssignVisitor;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoFuncAssignVisitor;

fn check_target(target: &AssignmentTarget, span: Span, ctx: &mut Context) {
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => {
      if let Some(BindingKind::Function) = ctx.binding_kind_of_ident_ref(ident)
      {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          NoFuncAssignMessage::Unexpected,
          NoFuncAssignHint::RemoveOrRework,
        );
      }
    }
    AssignmentTarget::ArrayAssignmentTarget(arr) => {
      check_array_target(arr, span, ctx);
    }
    AssignmentTarget::ObjectAssignmentTarget(obj) => {
      check_object_target(obj, span, ctx);
    }
    _ => {}
  }
}

fn check_array_target(
  arr: &ArrayAssignmentTarget,
  span: Span,
  ctx: &mut Context,
) {
  for elem in arr.elements.iter().flatten() {
    match elem {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_def) => {
        check_target(&with_def.binding, span, ctx);
      }
      _ => {
        if let Some(target) = elem.as_assignment_target() {
          check_target(target, span, ctx);
        }
      }
    }
  }
  if let Some(rest) = &arr.rest {
    check_target(&rest.target, span, ctx);
  }
}

fn check_object_target(
  obj: &ObjectAssignmentTarget,
  span: Span,
  ctx: &mut Context,
) {
  for prop in obj.properties.iter() {
    match prop {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ident) => {
        if let Some(BindingKind::Function) =
          ctx.binding_kind_of_ident_ref(&ident.binding)
        {
          ctx.add_diagnostic_with_hint(
            span,
            CODE,
            NoFuncAssignMessage::Unexpected,
            NoFuncAssignHint::RemoveOrRework,
          );
        }
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(kv) => {
        match &kv.binding {
          AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
            with_def,
          ) => {
            check_target(&with_def.binding, span, ctx);
          }
          _ => {
            if let Some(target) = kv.binding.as_assignment_target() {
              check_target(target, span, ctx);
            }
          }
        }
      }
    }
  }
  if let Some(rest) = &obj.rest {
    check_target(&rest.target, span, ctx);
  }
}

impl Handler<'_> for NoFuncAssignVisitor {
  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    check_target(&assign_expr.left, assign_expr.span, ctx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.13.0/tests/lib/rules/no-func-assign.js
  // MIT Licensed.

  #[test]
  fn no_func_assign_valid() {
    assert_lint_ok! {
      NoFuncAssign,
      "function foo() { var foo = bar; }",
      "function foo(foo) { foo = bar; }",
      "function foo() { var foo; foo = bar; }",
      "var foo = () => {}; foo = bar;",
      "var foo = function() {}; foo = bar;",
      "var foo = function() { foo = bar; };",
      "import bar from 'bar'; function foo() { var foo = bar; }",
    };
  }

  #[test]
  fn no_func_assign_invalid() {
    assert_lint_err! {
      NoFuncAssign,
      "function foo() {}; foo = bar;": [
        {
          col: 19,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { foo = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "foo = bar; function foo() { };": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "[foo] = bar; function foo() { }": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "({x: foo = 0} = bar); function foo() { };": [
        {
          col: 1,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { [foo] = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "(function() { ({x: foo = 0} = bar); function foo() { }; })();": [
        {
          col: 15,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "var a = function foo() { foo = 123; };": [
        {
          col: 25,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      r#"
const a = "a";
const unused = "unused";

function asdf(b: number, c: string): number {
    console.log(a, b);
    debugger;
    return 1;
}

asdf = "foobar";
      "#: [
        {
          col: 0,
          line: 11,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],

      // nested
      r#"
function foo() {}
let a;
a = () => {
  foo = 42;
};
      "#: [
        {
          line: 5,
          col: 2,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
    };
  }
}
