// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::SourceRanged;
use deno_ast::view::{CallExpr, Callee, Expr, MemberProp};
use std::sync::Arc;

const BANNED_PROPERTIES: &[&str] =
  &["hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable"];

#[derive(Debug)]
pub struct NoPrototypeBuiltins;

const CODE: &str = "no-prototype-builtins";

fn get_message(prop: &str) -> String {
  format!(
    "Access to Object.prototype.{} is not allowed from target object",
    prop
  )
}

impl LintRule for NoPrototypeBuiltins {
  fn new() -> Arc<Self> {
    Arc::new(NoPrototypeBuiltins)
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
    NoPrototypeBuiltinsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_prototype_builtins.md")
  }
}

struct NoPrototypeBuiltinsHandler;

impl Handler for NoPrototypeBuiltinsHandler {
  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    let member_expr = match call_expr.callee {
      Callee::Expr(boxed_expr) => match boxed_expr {
        Expr::Member(member_expr) => member_expr,
        _ => return,
      },
      Callee::Super(_) | Callee::Import(_) => return,
    };

    if let MemberProp::Ident(ident) = member_expr.prop {
      let prop_name = ident.sym().as_ref();
      if BANNED_PROPERTIES.contains(&prop_name) {
        ctx.add_diagnostic(call_expr.range(), CODE, get_message(prop_name));
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_prototype_builtins_valid() {
    assert_lint_ok! {
      NoPrototypeBuiltins,
      r#"
  Object.prototype.hasOwnProperty.call(foo, "bar");
  Object.prototype.isPrototypeOf.call(foo, "bar");
  Object.prototype.propertyIsEnumerable.call(foo, "bar");
  Object.prototype.hasOwnProperty.apply(foo, ["bar"]);
  Object.prototype.isPrototypeOf.apply(foo, ["bar"]);
  Object.prototype.propertyIsEnumerable.apply(foo, ["bar"]);
  hasOwnProperty(foo, "bar");
  isPrototypeOf(foo, "bar");
  propertyIsEnumerable(foo, "bar");
  ({}.hasOwnProperty.call(foo, "bar"));
  ({}.isPrototypeOf.call(foo, "bar"));
  ({}.propertyIsEnumerable.call(foo, "bar"));
  ({}.hasOwnProperty.apply(foo, ["bar"]));
  ({}.isPrototypeOf.apply(foo, ["bar"]));
  ({}.propertyIsEnumerable.apply(foo, ["bar"]));
      "#,
    };
  }

  #[test]
  fn no_prototype_builtins_invalid() {
    assert_lint_err! {
      NoPrototypeBuiltins,
      "foo.hasOwnProperty('bar');": [{col: 0, message: get_message("hasOwnProperty")}],
      "foo.isPrototypeOf('bar');": [{col: 0, message: get_message("isPrototypeOf")}],
      "foo.propertyIsEnumerable('bar');": [{col: 0, message: get_message("propertyIsEnumerable")}],
      "foo.bar.baz.hasOwnProperty('bar');": [{col: 0, message: get_message("hasOwnProperty")}],
    }
  }
}
