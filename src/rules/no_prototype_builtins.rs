// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;

use deno_ast::swc::ast::CallExpr;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ExprOrSuper;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;

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
  fn new() -> Box<Self> {
    Box::new(NoPrototypeBuiltins)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoPrototypeBuiltinsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_prototype_builtins.md")
  }
}

struct NoPrototypeBuiltinsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoPrototypeBuiltinsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoPrototypeBuiltinsVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    let member_expr = match &call_expr.callee {
      ExprOrSuper::Expr(boxed_expr) => match &**boxed_expr {
        Expr::Member(member_expr) => {
          if member_expr.computed {
            return;
          }
          member_expr
        }
        _ => return,
      },
      ExprOrSuper::Super(_) => return,
    };

    if let Expr::Ident(ident) = &*member_expr.prop {
      let prop_name = ident.sym.as_ref();
      if BANNED_PROPERTIES.contains(&prop_name) {
        self.context.add_diagnostic(
          call_expr.span,
          CODE,
          get_message(prop_name),
        );
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
