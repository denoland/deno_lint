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

pub struct NoExAssign;

impl LintRule for NoExAssign {
  fn new() -> Box<Self> {
    Box::new(NoExAssign)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(&module, &module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = NoExAssignVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoExAssignVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl NoExAssignVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }
}

impl Visit for NoExAssignVisitor {
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
      if binding.kind == BindingKind::CatchClause {
        self.context.add_diagnostic(
          assign_expr.span,
          "noExAssign",
          "Reassigning exception parameter is not allowed",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn no_ex_assign_ok() {
    test_lint(
      "no_ex_assign",
      r#"
try {} catch { e = 1; }
try {} catch (ex) { something = 1; }
try {} catch (ex) { return 1; }
      "#,
      vec![NoExAssign::new()],
      json!([]),
    )
  }

  #[test]
  fn no_ex_assign() {
    test_lint(
      "no_ex_assign",
      r#"
try {} catch (e) { e = 1; }
try {} catch (ex) { ex = 1; }
try {} catch (ex) { [ex] = []; }
try {} catch ({message}) { message = 1; }
      "#,
      vec![NoExAssign::new()],
      json!([{
        "code": "noExAssign",
        "message": "Reassigning exception parameter is not allowed",
        "location": {
          "filename": "no_ex_assign",
          "line": 11,
          "col": 0,
        }
      },{
        "code": "noExAssign",
        "message": "Reassigning exception parameter is not allowed",
        "location": {
          "filename": "no_ex_assign",
          "line": 11,
          "col": 0,
        }
      },{
        "code": "noExAssign",
        "message": "Reassigning exception parameter is not allowed",
        "location": {
          "filename": "no_ex_assign",
          "line": 11,
          "col": 0,
        }
      },{
        "code": "noExAssign",
        "message": "Reassigning exception parameter is not allowed",
        "location": {
          "filename": "no_ex_assign",
          "line": 11,
          "col": 0,
        }
      }]),
    )
  }
}
