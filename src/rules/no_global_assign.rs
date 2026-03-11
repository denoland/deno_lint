// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use crate::globals::GLOBALS;
use deno_ast::oxc::ast::ast::{
  AssignmentExpression, AssignmentTarget, AssignmentTargetMaybeDefault,
  AssignmentTargetProperty, Program, SimpleAssignmentTarget, UpdateExpression,
};
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct NoGlobalAssign;

const CODE: &str = "no-global-assign";

#[derive(Display)]
enum NoGlobalAssignMessage {
  #[display(fmt = "Assignment to global is not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoGlobalAssignHint {
  #[display(fmt = "Remove the assignment to the global variable")]
  Remove,
}

impl LintRule for NoGlobalAssign {
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
    let mut handler = NoGlobalAssignVisitor;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoGlobalAssignVisitor;

impl NoGlobalAssignVisitor {
  fn check(
    &mut self,
    span: Span,
    name: &str,
    reference_id: Option<deno_ast::oxc::semantic::ReferenceId>,
    ctx: &mut Context,
  ) {
    // Check if the identifier resolves to a local binding via OXC scoping.
    if let Some(ref_id) = reference_id {
      let reference = ctx.scoping().get_reference(ref_id);
      if reference.symbol_id().is_some() {
        return; // Resolved to a local binding, not a global
      }
    } else if ctx.scope().var_by_name(name).is_some() {
      return;
    }

    // We only care about globals.
    let maybe_global = GLOBALS.iter().find(|(gname, _)| *gname == name);

    if let Some(global) = maybe_global {
      // If global can be overwritten then don't need to report anything
      if !global.1 {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          NoGlobalAssignMessage::NotAllowed,
          NoGlobalAssignHint::Remove,
        );
      }
    }
  }

  fn check_target(
    &mut self,
    target: &AssignmentTarget,
    assign_span: Span,
    ctx: &mut Context,
  ) {
    match target {
      AssignmentTarget::AssignmentTargetIdentifier(ident) => {
        self.check(ident.span, ident.name.as_str(), ident.reference_id.get(), ctx);
      }
      AssignmentTarget::ObjectAssignmentTarget(obj) => {
        for prop in obj.properties.iter() {
          match prop {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
              ident,
            ) => {
              self.check(
                ident.binding.span,
                ident.binding.name.as_str(),
                ident.binding.reference_id.get(),
                ctx,
              );
            }
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(kv) => {
              self.check_target_maybe_default(&kv.binding, assign_span, ctx);
            }
          }
        }
        if let Some(rest) = &obj.rest {
          self.check_target(&rest.target, assign_span, ctx);
        }
      }
      AssignmentTarget::ArrayAssignmentTarget(arr) => {
        for elem in arr.elements.iter().flatten() {
          self.check_target_maybe_default(elem, assign_span, ctx);
        }
        if let Some(rest) = &arr.rest {
          self.check_target(&rest.target, assign_span, ctx);
        }
      }
      _ => {}
    }
  }

  fn check_target_maybe_default(
    &mut self,
    target: &AssignmentTargetMaybeDefault,
    assign_span: Span,
    ctx: &mut Context,
  ) {
    match target {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_def) => {
        self.check_target(&with_def.binding, assign_span, ctx);
      }
      _ => {
        if let Some(t) = target.as_assignment_target() {
          self.check_target(t, assign_span, ctx);
        }
      }
    }
  }
}

impl Handler<'_> for NoGlobalAssignVisitor {
  fn assignment_expression(
    &mut self,
    e: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    self.check_target(&e.left, e.span, ctx);
  }

  fn update_expression(&mut self, e: &UpdateExpression, ctx: &mut Context) {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(i) = &e.argument {
      self.check(e.span, i.name.as_str(), i.reference_id.get(), ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_global_assign_valid() {
    assert_lint_ok! {
      NoGlobalAssign,
      "string = 'hello world';",
      "var string;",
      "top = 0;",
      "require = 0;",
      "onmessage = function () {};",
      "let Array = 0; Array = 42;",
      r#"
let Boolean = true;
function foo() {
  Boolean = false;
}
      "#,
    };
  }

  #[test]
  fn no_global_assign_invalid() {
    assert_lint_err! {
      NoGlobalAssign,
      "String = 'hello world';": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "String++;": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "({Object = 0, String = 0} = {});": [
        {
          col: 2,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        },
        {
          col: 14,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "Array = 1;": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      r#"
function foo() {
  let Boolean = false;
  Boolean = true;
}
Boolean = true;
      "#: [
        {
          col: 0,
          line: 6,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        },
      ],
    };
  }
}
