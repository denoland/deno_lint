// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  AssignmentExpression, AssignmentTarget, AssignmentTargetMaybeDefault,
  AssignmentTargetProperty, ArrayAssignmentTarget, ObjectAssignmentTarget,
  Program, SimpleAssignmentTarget, UpdateExpression,
};
use deno_ast::oxc::span::Span;
use deno_ast::BindingKind;
use derive_more::Display;

#[derive(Debug)]
pub struct NoConstAssign;

const CODE: &str = "no-const-assign";

#[derive(Display)]
enum NoConstantAssignMessage {
  #[display(fmt = "Reassigning constant variable is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoConstantAssignHint {
  #[display(
    fmt = "Change `const` declaration to `let` or double check the correct variable is used"
  )]
  Remove,
}
impl LintRule for NoConstAssign {
  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoConstAssignHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoConstAssignHandler;

fn check_ident_ref_for_const(
  range: Span,
  ident: &deno_ast::oxc::ast::ast::IdentifierReference,
  ctx: &mut Context,
) {
  if let Some(BindingKind::Const) = ctx.binding_kind_of_ident_ref(ident) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      NoConstantAssignMessage::Unexpected,
      NoConstantAssignHint::Remove,
    );
  }
}

fn check_assignment_target(
  target: &AssignmentTarget,
  range: Span,
  ctx: &mut Context,
) {
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => {
      check_ident_ref_for_const(range, ident, ctx);
    }
    AssignmentTarget::ArrayAssignmentTarget(array) => {
      check_array_assignment_target(array, range, ctx);
    }
    AssignmentTarget::ObjectAssignmentTarget(object) => {
      check_obj_assignment_target(object, range, ctx);
    }
    _ => {}
  }
}

fn check_maybe_default(
  target: &AssignmentTargetMaybeDefault,
  range: Span,
  ctx: &mut Context,
) {
  match target {
    AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(d) => {
      check_assignment_target(&d.binding, range, ctx);
    }
    AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(ident) => {
      check_ident_ref_for_const(range, ident, ctx);
    }
    _ => {}
  }
}

fn check_obj_assignment_target(
  object: &ObjectAssignmentTarget,
  range: Span,
  ctx: &mut Context,
) {
  for prop in &object.properties {
    match prop {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
        assign_prop,
      ) => {
        check_ident_ref_for_const(
          assign_prop.span,
          &assign_prop.binding,
          ctx,
        );
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(kv_prop) => {
        check_maybe_default(&kv_prop.binding, range, ctx);
      }
    }
  }
}

fn check_array_assignment_target(
  array: &ArrayAssignmentTarget,
  range: Span,
  ctx: &mut Context,
) {
  for elem in array.elements.iter().flatten() {
    check_maybe_default(elem, range, ctx);
  }
}

impl Handler<'_> for NoConstAssignHandler {
  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    check_assignment_target(&assign_expr.left, assign_expr.span, ctx);
  }

  fn update_expression(
    &mut self,
    update_expr: &UpdateExpression,
    ctx: &mut Context,
  ) {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) =
      &update_expr.argument
    {
      check_ident_ref_for_const(update_expr.span, ident, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_const_assign_valid() {
    assert_lint_ok! {
      NoConstAssign,
      r#"
      const x = 0; { let x; x = 1; }
      const x = 0; function a(x) { x = 1; }
      const x = 0; foo(x);
      for (const x in [1,2,3]) { foo(x); }
      for (const x of [1,2,3]) { foo(x); }
      const x = {key: 0}; x.key = 1;
      if (true) {const a = 1} else { a = 2};
      // ignores non constant.
      var x = 0; x = 1;
      let x = 0; x = 1;
      function x() {} x = 1;
      function foo(x) { x = 1; }
      class X {} X = 1;
      try {} catch (x) { x = 1; }
      Deno.test("test function", function(){
        const a = 1;
      });
      Deno.test("test another function", function(){
        a=2;
      });

      Deno.test({
        name : "test object",
        fn() : Promise<void> {
          const a = 1;
        }
      });

      Deno.test({
        name : "test another object",
        fn() : Promise<void> {
         a = 2;
        }
      });

      let obj = {
        get getter(){
          const a = 1;
          return a;
        }
        ,
        set setter(x){
          a = 2;
        }
      }
      "#,
    };
  }

  #[test]
  fn no_const_assign_invalid() {
    assert_lint_err! {
      NoConstAssign,
      r#"const x = 0; x = 1;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const {a: x} = {a: 0}; x = 1;"#: [
      {
        col: 23,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ({x} = {x: 1});"#: [
      {
        col: 15,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ({a: x = 1} = {});"#: [
      {
        col: 14,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; x += 1;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ++x;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; function foo() { x = x + 1; }"#: [
      {
        col: 30,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; function foo(a) { x = a; }"#: [
      {
        col: 31,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"for (const i = 0; i < 10; ++i) {}"#: [
      {
        col: 26,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; while (true) { x = x + 1; }"#: [
      {
        col: 28,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
switch (char) {
  case "a":
    const a = true;
  break;
  case "b":
    a = false;
  break;
}"#: [
      {
        line: 7,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
try {
  const a = 1;
  a = 2;
} catch (e) {}"#:[
      {
        line: 4,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
if (true) {
  const a = 1;
  if (false) {
    a = 2;
  } else {
    a = 2;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      },
      {
        line: 7,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
for (const a of [1, 2, 3]) {
  a = 0;
}"#:[
      {
        line: 3,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
for (const a in [1, 2, 3]) {
  a = 0;
}"#:[
      {
        line: 3,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
while (true) {
  const a = 1;
  while (a == 1) {
    a = 2;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
const lambda = () => {
  const a = 1;
  {
    a = 1;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
class URL {
  get port(){
    const port = 80;
    port = 3000;
    return port;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
declare module "foo" {
  const a = 1;
  a=2;
}"#:[
      {
        line: 4,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      "const x = 0  ; x = 1; x = 2;": [
      {
        line: 1,
        col: 15,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      },
      {
        line: 1,
        col: 22,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
    }
  }
}
