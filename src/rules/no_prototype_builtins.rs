// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::CallExpr;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprOrSuper;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub const BANNED_PROPERTIES: &[&str] =
  &["hasOwnProperty", "isPrototypeOf", "propertyIsEnumberable"];

pub struct NoPrototypeBuiltins;

impl LintRule for NoPrototypeBuiltins {
  fn new() -> Box<Self> {
    Box::new(NoPrototypeBuiltins)
  }

  fn code(&self) -> &'static str {
    "noPrototypeBuiltins"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoPrototypeBuiltinsVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoPrototypeBuiltinsVisitor {
  context: Context,
}

impl NoPrototypeBuiltinsVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoPrototypeBuiltinsVisitor {
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
      let prop_name = ident.sym.to_string();

      if BANNED_PROPERTIES.contains(&prop_name.as_str()) {
        self.context.add_diagnostic(
          call_expr.span,
          "noPrototypeBuiltins",
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
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.hasOwnProperty("bar");"#,
      "noPrototypeBuiltins",
      0,
    );
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.isPrototypeOf("bar");"#,
      "noPrototypeBuiltins",
      0,
    );
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.propertyIsEnumberable("bar");"#,
      "noPrototypeBuiltins",
      0,
    );
    assert_lint_err::<NoPrototypeBuiltins>(
      r#"foo.bar.baz.hasOwnProperty("bar");"#,
      "noPrototypeBuiltins",
      0,
    );
  }
}
