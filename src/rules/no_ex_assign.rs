// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use deno_ast::BindingKind;
use derive_more::Display;

#[derive(Debug)]
pub struct NoExAssign;

const CODE: &str = "no-ex-assign";

#[derive(Display)]
enum NoExAssignMessage {
  #[display(fmt = "Reassigning exception parameter is not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoExAssignHint {
  #[display(fmt = "Use a different variable for the assignment")]
  UseDifferent,
}

impl LintRule for NoExAssign {
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
    let mut handler = NoExAssignHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoExAssignHandler;

fn check_ident_ref_for_catch(
  range: Span,
  ident: &IdentifierReference,
  ctx: &mut Context,
) {
  if let Some(BindingKind::CatchClause) = ctx.binding_kind_of_ident_ref(ident) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      NoExAssignMessage::NotAllowed,
      NoExAssignHint::UseDifferent,
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
      check_ident_ref_for_catch(range, ident, ctx);
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
        check_ident_ref_for_catch(
          assign_prop.binding.span,
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

fn check_maybe_default(
  target: &AssignmentTargetMaybeDefault,
  range: Span,
  ctx: &mut Context,
) {
  match target {
    AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_default) => {
      check_assignment_target(&with_default.binding, range, ctx);
    }
    AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(ident) => {
      check_ident_ref_for_catch(range, ident, ctx);
    }
    AssignmentTargetMaybeDefault::ArrayAssignmentTarget(array) => {
      check_array_assignment_target(array, range, ctx);
    }
    AssignmentTargetMaybeDefault::ObjectAssignmentTarget(object) => {
      check_obj_assignment_target(object, range, ctx);
    }
    _ => {}
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

impl Handler<'_> for NoExAssignHandler {
  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    check_assignment_target(&assign_expr.left, assign_expr.span, ctx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_ex_assign_valid() {
    assert_lint_ok! {
      NoExAssign,
      r#"
try {} catch { e = 1; }
try {} catch (ex) { something = 1; }
try {} catch (ex) { return 1; }
function foo() { try { } catch (e) { return false; } }
      "#,
    };
  }

  #[test]
  fn no_ex_assign_invalid() {
    assert_lint_err! {
      NoExAssign,
      r#"try {} catch (e) { e = 1; }"#: [
        {
          col: 19,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { ex = 1; }"#: [
        {
          col: 20,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { [ex] = []; }"#: [
        {
          col: 20,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch (ex) { ({x: ex = 0} = {}); }"#: [
        {
          col: 21,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
      r#"try {} catch ({message}) { message = 1; }"#: [
        {
          col: 27,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],

      // nested
      r#"a = () => { try {} catch (e) { e = 1; } };"#: [
        {
          col: 31,
          message: NoExAssignMessage::NotAllowed,
          hint: NoExAssignHint::UseDifferent,
        },
      ],
    };
  }
}
