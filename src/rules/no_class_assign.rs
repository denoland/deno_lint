// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::PatOrExpr;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoClassAssign;

impl LintRule for NoClassAssign {
  fn new() -> Box<Self> {
    Box::new(NoClassAssign)
  }

  fn code(&self) -> &'static str {
    "no-class-assign"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecmascript::ast::Module) {
    let mut visitor = NoClassAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoClassAssignVisitor {
  context: Arc<Context>,
}

impl NoClassAssignVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoClassAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let name = match &assign_expr.left {
      PatOrExpr::Expr(_) => return,
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => &ident.sym,
        _ => return,
      },
    };

    let scope = self.context.root_scope.get_scope_for_span(assign_expr.span);
    if let Some(BindingKind::Class) = scope.get_binding(name) {
      self.context.add_diagnostic(
        assign_expr.span,
        "no-class-assign",
        "Reassigning class declaration is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line_n;
  use crate::test_util::assert_lint_ok;

  #[test]
  fn no_class_assign_ok() {
    assert_lint_ok::<NoClassAssign>(
      r#"
class A {}

class B {
  foo(A) {
    A = "foobar";
  }
}

class C {
  bar() {
    let B;
    B = "bar";
  }
}

let x = "x";
x = "xx";
var y = "y";
y = "yy";
      "#,
    );
  }

  #[test]
  fn no_class_assign() {
    assert_lint_err_on_line_n::<NoClassAssign>(
      r#"
class A {}
A = 0;

class B {
  foo() {
    B = 0;
  }
}

class C {}
C = 10;
C = 20;
      "#,
      vec![(3, 0), (7, 4), (12, 0), (13, 0)],
    );
  }
}
