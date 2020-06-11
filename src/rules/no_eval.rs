// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::CallExpr;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ExprOrSuper;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEval;

impl LintRule for NoEval {
  fn new() -> Box<Self> {
    Box::new(NoEval)
  }

  fn code(&self) -> &'static str {
    "no-eval"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEvalVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoEvalVisitor {
  context: Context,
}

impl NoEvalVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEvalVisitor {
  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.to_string();
        if name == "eval" {
          self.context.add_diagnostic(
            call_expr.span,
            "no-eval",
            "`eval` call is not allowed",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_eval_test() {
    assert_lint_err::<NoEval>(r#"eval("123");"#, 0)
  }
}
