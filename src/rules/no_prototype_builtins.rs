// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub const BANNED_PROPERTIES: &[&str] =
  &["hasOwnProperty", "isPrototypeOf", "propertyIsEnumberable"];

use std::sync::Arc;

pub struct NoPrototypeBuiltins;

impl LintRule for NoPrototypeBuiltins {
  fn new() -> Box<Self> {
    Box::new(NoPrototypeBuiltins)
  }

  fn code(&self) -> &'static str {
    "no-prototype-builtins"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoPrototypeBuiltinsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoPrototypeBuiltinsVisitor {
  context: Arc<Context>,
}

impl NoPrototypeBuiltinsVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoPrototypeBuiltinsVisitor {
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
          "no-prototype-builtins",
          &format!(
            "Access to Object.prototype.{} is not allowed from target object",
            prop_name
          ),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_prototype_builtins_ok() {
    assert_lint_ok::<NoPrototypeBuiltins>(
      r#"
  Object.prototype.hasOwnProperty.call(foo, "bar");
  Object.prototype.isPrototypeOf.call(foo, "bar");
  Object.prototype.propertyIsEnumberable.call(foo, "bar");
  Object.prototype.hasOwnProperty.apply(foo, ["bar"]);
  Object.prototype.isPrototypeOf.apply(foo, ["bar"]);
  Object.prototype.propertyIsEnumberable.apply(foo, ["bar"]);
  hasOwnProperty(foo, "bar");
  isPrototypeOf(foo, "bar");
  propertyIsEnumberable(foo, "bar");
  ({}.hasOwnProperty.call(foo, "bar"));
  ({}.isPrototypeOf.call(foo, "bar"));
  ({}.propertyIsEnumberable.call(foo, "bar"));
  ({}.hasOwnProperty.apply(foo, ["bar"]));
  ({}.isPrototypeOf.apply(foo, ["bar"]));
  ({}.propertyIsEnumberable.apply(foo, ["bar"]));
      "#,
    );
  }

  #[test]
  fn no_prototype_builtins() {
    assert_lint_err::<NoPrototypeBuiltins>(r#"foo.hasOwnProperty("bar");"#, 0);
    assert_lint_err::<NoPrototypeBuiltins>(r#"foo.isPrototypeOf("bar");"#, 0);
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.propertyIsEnumberable("bar");"#,
      0,
    );
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.bar.baz.hasOwnProperty("bar");"#,
      0,
    );
  }
}
