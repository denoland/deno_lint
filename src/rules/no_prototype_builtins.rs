// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{CallExpression, Expression, Program};

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
    let mut handler = NoPrototypeBuiltinsHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoPrototypeBuiltinsHandler;

impl Handler<'_> for NoPrototypeBuiltinsHandler {
  fn call_expression(
    &mut self,
    call_expr: &CallExpression,
    ctx: &mut Context,
  ) {
    if let Expression::StaticMemberExpression(member_expr) =
      &call_expr.callee
    {
      let prop_name = member_expr.property.name.as_str();
      if BANNED_PROPERTIES.contains(&prop_name) {
        ctx.add_diagnostic(call_expr.span, CODE, get_message(prop_name));
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
