// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::scopes::ScopeManager;
use crate::scopes::ScopeVisitor;
use swc_ecma_ast::AssignExpr;
use swc_ecma_ast::Pat;
use swc_ecma_ast::PatOrExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoClassAssign;

impl LintRule for NoClassAssign {
  fn new() -> Box<Self> {
    Box::new(NoClassAssign)
  }

  fn code(&self) -> &'static str {
    "no-class-assign"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(&module, &module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = NoClassAssignVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoClassAssignVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl NoClassAssignVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }
}

impl Visit for NoClassAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ident = match &assign_expr.left {
      PatOrExpr::Expr(_) => return,
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => return,
      },
    };

    let scope = self.scope_manager.get_scope_for_span(assign_expr.span);
    if let Some(binding) = self.scope_manager.get_binding(scope, &ident) {
      if binding.kind == BindingKind::Class {
        self.context.add_diagnostic(
          assign_expr.span,
          "no-class-assign",
          "Reassigning class declaration is not allowed",
        );
      }
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
