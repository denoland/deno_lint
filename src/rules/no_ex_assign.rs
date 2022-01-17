// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{
  ArrayPat, AssignExpr, Expr, Ident, ObjectPat, ObjectPatProp, Pat, PatOrExpr,
};
use deno_ast::BindingKind;
use derive_more::Display;
use std::sync::Arc;

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
  fn new() -> Arc<Self> {
    Arc::new(NoExAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoExAssignHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_ex_assign.md")
  }
}

struct NoExAssignHandler;

fn check_pat(pat: &Pat, span: Span, ctx: &mut Context) {
  match pat {
    Pat::Ident(ident) => {
      check_scope_for_const(span, ident.id, ctx);
    }
    Pat::Assign(assign) => {
      check_pat(&assign.left, span, ctx);
    }
    Pat::Array(array) => {
      check_array_pat(array, span, ctx);
    }
    Pat::Object(object) => {
      check_obj_pat(object, span, ctx);
    }
    _ => {}
  }
}

fn check_obj_pat(object: &ObjectPat, span: Span, ctx: &mut Context) {
  if !object.props.is_empty() {
    for prop in object.props.iter() {
      if let ObjectPatProp::Assign(assign_prop) = prop {
        check_scope_for_const(assign_prop.key.span(), assign_prop.key, ctx);
      } else if let ObjectPatProp::KeyValue(kv_prop) = prop {
        check_pat(&kv_prop.value, span, ctx);
      }
    }
  }
}

fn check_array_pat(array: &ArrayPat, span: Span, ctx: &mut Context) {
  if !array.elems.is_empty() {
    for elem in array.elems.iter().flatten() {
      check_pat(elem, span, ctx);
    }
  }
}

fn check_scope_for_const(span: Span, name: &Ident, ctx: &mut Context) {
  if let Some(v) = ctx.scope().var_by_ident(name) {
    if let BindingKind::CatchClause = v.kind() {
      ctx.add_diagnostic_with_hint(
        span,
        CODE,
        NoExAssignMessage::NotAllowed,
        NoExAssignHint::UseDifferent,
      );
    }
  }
}

impl Handler for NoExAssignHandler {
  fn assign_expr(&mut self, assign_expr: &AssignExpr, ctx: &mut Context) {
    match &assign_expr.left {
      PatOrExpr::Expr(pat_expr) => {
        if let Expr::Ident(ident) = pat_expr {
          check_scope_for_const(assign_expr.span(), ident, ctx);
        }
      }
      PatOrExpr::Pat(boxed_pat) => {
        check_pat(boxed_pat, assign_expr.span(), ctx)
      }
    };
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
