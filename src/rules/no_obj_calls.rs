// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::CallExpr;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::ExprOrSuper;
use crate::swc_ecma_ast::NewExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoObjCalls;

impl LintRule for NoObjCalls {
  fn new() -> Box<Self> {
    Box::new(NoObjCalls)
  }

  fn code(&self) -> &'static str {
    "no-obj-call"
  }

  fn lint_module(&self, context: Context, module: &swc_ecma_ast::Module) {
    let mut visitor = NoObjCallsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoObjCallsVisitor {
  context: Context,
}

impl NoObjCallsVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_callee(&self, callee_name: String, span: Span) {
    match callee_name.as_ref() {
      "Math" | "JSON" | "Reflect" | "Atomics" => {
        self.context.add_diagnostic(
          span,
          "no-obj-call",
          format!("`{}` call as function is not allowed", callee_name).as_ref(),
        );
      }
      _ => {}
    }
  }
}

impl Visit for NoObjCallsVisitor {
  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.to_string();
        self.check_callee(name, call_expr.span);
      }
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.to_string();
      self.check_callee(name, new_expr.span);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn test_no_call_math() {
    assert_lint_err::<NoObjCalls>(r#"Math();"#, 0)
  }

  #[test]
  fn test_no_new_math() {
    assert_lint_err::<NoObjCalls>(r#"new Math();"#, 0)
  }

  #[test]
  fn test_no_call_json() {
    assert_lint_err::<NoObjCalls>(r#"JSON();"#, 0)
  }

  #[test]
  fn test_no_new_json() {
    assert_lint_err::<NoObjCalls>(r#"new JSON();"#, 0)
  }

  #[test]
  fn test_no_call_reflect() {
    assert_lint_err::<NoObjCalls>(r#"Reflect();"#, 0)
  }

  #[test]
  fn test_no_new_reflect() {
    assert_lint_err::<NoObjCalls>(r#"new Reflect();"#, 0)
  }

  #[test]
  fn test_no_call_atomicst() {
    assert_lint_err::<NoObjCalls>(r#"Atomics();"#, 0)
  }

  #[test]
  fn test_no_new_atomics() {
    assert_lint_err::<NoObjCalls>(r#"new Atomics();"#, 0)
  }

  #[test]
  fn test_math_func_ok() {
    assert_lint_ok::<NoObjCalls>("Math.PI * 2 * 3;");
  }

  #[test]
  fn test_new_json_ok() {
    assert_lint_ok::<NoObjCalls>("JSON.parse(\"{}\");");
  }

  #[test]
  fn test_reflect_get_ok() {
    assert_lint_ok::<NoObjCalls>("Reflect.get({ x: 1, y: 2 }, \"x\");");
  }

  #[test]
  fn test_atomic_load_ok() {
    assert_lint_ok::<NoObjCalls>("Atomics.load(foo, 0);");
  }
}
