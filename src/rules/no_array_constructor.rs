// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecma_ast::{CallExpr, Expr, ExprOrSpread, ExprOrSuper, NewExpr};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoArrayConstructor;

impl LintRule for NoArrayConstructor {
  fn new() -> Box<Self> {
    Box::new(NoArrayConstructor)
  }

  fn code(&self) -> &'static str {
    "noArrayConstructor"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoArrayConstructorVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoArrayConstructorVisitor {
  context: Context,
}

impl NoArrayConstructorVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  pub fn check_args(&self, args: Vec<ExprOrSpread>, span: Span) {
    if args.len() != 1 {
      self.context.add_diagnostic(
        span,
        "noArrayConstructor",
        "Array Constructor is not allowed",
      );
    }
  }
}

impl Visit for NoArrayConstructorVisitor {
  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.to_string();
      if name != "Array" {
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
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.to_string();
        if name != "Array" {
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
  fn no_array_constructor_invalid() {
    assert_lint_err::<NoArrayConstructor>("new Array", 0);
    assert_lint_err::<NoArrayConstructor>("new Array()", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(x, y)", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(0, 1, 2)", 0);
  }
}
