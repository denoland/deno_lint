// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::NewExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoObjCalls;

impl LintRule for NoObjCalls {
  fn new() -> Box<Self> {
    Box::new(NoObjCalls)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-obj-calls"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoObjCallsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoObjCallsVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoObjCallsVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_callee(&mut self, callee_name: impl AsRef<str>, span: Span) {
    let callee_name = callee_name.as_ref();
    match callee_name {
      "Math" | "JSON" | "Reflect" | "Atomics" => {
        self.context.add_diagnostic(
          span,
          "no-obj-calls",
          format!("`{}` call as function is not allowed", callee_name),
        );
      }
      _ => {}
    }
  }
}

impl<'c> Visit for NoObjCallsVisitor<'c> {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        self.check_callee(&ident.sym, call_expr.span);
      }
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      self.check_callee(&ident.sym, new_expr.span);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_obj_calls_valid() {
    assert_lint_ok! {
      NoObjCalls,
      "Math.PI * 2 * 3;",
      "JSON.parse(\"{}\");",
      "Reflect.get({ x: 1, y: 2 }, \"x\");",
      "Atomics.load(foo, 0);",
    };
  }

  #[test]
  fn no_obj_calls_invalid() {
    assert_lint_err::<NoObjCalls>(r#"Math();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"new Math();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"JSON();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"new JSON();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"Reflect();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"new Reflect();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"Atomics();"#, 0);
    assert_lint_err::<NoObjCalls>(r#"new Atomics();"#, 0);
  }
}
