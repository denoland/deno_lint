// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSpread, ExprOrSuper, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct NoArrayConstructor;

impl LintRule for NoArrayConstructor {
  fn new() -> Box<Self> {
    Box::new(NoArrayConstructor)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-array-constructor"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoArrayConstructorVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoArrayConstructorVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoArrayConstructorVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_args(&mut self, args: Vec<ExprOrSpread>, span: Span) {
    if args.len() != 1 {
      self.context.add_diagnostic(
        span,
        "no-array-constructor",
        "Array Constructor is not allowed",
      );
    }
  }
}

impl<'c> Visit for NoArrayConstructorVisitor<'c> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    new_expr.visit_children_with(self);
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.as_ref();
      if name != "Array" {
        return;
      }
      if new_expr.type_args.is_some() {
        return;
      }
      match &new_expr.args {
        Some(args) => {
          self.check_args(args.to_vec(), new_expr.span);
        }
        None => self.check_args(vec![], new_expr.span),
      };
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    call_expr.visit_children_with(self);
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.as_ref();
        if name != "Array" {
          return;
        }
        if call_expr.type_args.is_some() {
          return;
        }

        self.check_args((&*call_expr.args).to_vec(), call_expr.span);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_array_constructor_call_valid() {
    assert_lint_ok_n::<NoArrayConstructor>(vec![
      "Array(x)",
      "Array(9)",
      "Array.foo()",
      "foo.Array()",
    ]);
  }

  #[test]
  fn no_array_constructor_new_valid() {
    assert_lint_ok_n::<NoArrayConstructor>(vec![
      "new Array(x)",
      "new Array(9)",
      "new foo.Array()",
      "new Array.foo",
    ]);
  }

  #[test]
  fn no_array_constructor_typescript_valid() {
    assert_lint_ok_n::<NoArrayConstructor>(vec![
      "new Array<Foo>(1, 2, 3);",
      "new Array<Foo>()",
      "Array<Foo>(1, 2, 3);",
      "Array<Foo>();",
    ]);
  }

  #[test]
  fn no_array_constructor_invalid() {
    assert_lint_err::<NoArrayConstructor>("new Array", 0);
    assert_lint_err::<NoArrayConstructor>("new Array()", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(x, y)", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(0, 1, 2)", 0);
    // nested
    assert_lint_err_on_line::<NoArrayConstructor>(
      r#"
const a = new class {
  foo() {
    let arr = new Array();
  }
}();
"#,
      4,
      14,
    );
    assert_lint_err_on_line::<NoArrayConstructor>(
      r#"
const a = (() => {
  let arr = new Array();
})();
"#,
      3,
      12,
    );
  }
}
