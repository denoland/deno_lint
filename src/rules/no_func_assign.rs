// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::scopes::ScopeVisitor;
use swc_ecma_ast::AssignExpr;
use swc_ecma_ast::Pat;
use swc_ecma_ast::PatOrExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoFuncAssign;

impl LintRule for NoFuncAssign {
  fn new() -> Box<Self> {
    Box::new(NoFuncAssign)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_manager = ScopeVisitor::default();
    scope_manager.visit_module(&module, &module);
    let mut visitor = NoFuncAssignVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoFuncAssignVisitor {
  context: Context,
  scope_manager: ScopeVisitor,
}

impl NoFuncAssignVisitor {
  pub fn new(context: Context, scope_manager: ScopeVisitor) -> Self {
    Self {
      context,
      scope_manager,
    }
  }
}

impl Visit for NoFuncAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ident = match &assign_expr.left {
      PatOrExpr::Expr(_) => return,
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => ident.sym.to_string(),
        _ => return,
      },
    };

    dbg!(self.scope_manager.get_current_scope());
    if let Some(binding) = self.scope_manager.get_binding(&ident) {
      if binding.kind == BindingKind::Function {
        self.context.add_diagnostic(
          assign_expr.span,
          "noFuncAssign",
          "Reassigning function declaration is not allowed",
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
  fn no_func_assign() {
    test_lint(
      "no_func_assign",
      r#"
const a = "a";
const unused = "unused";

function asdf(b: number, c: string): number {
    console.log(a, b);
    debugger;
    return 1;
}

asdf = "foobar";
      "#,
      vec![NoFuncAssign::new()],
      json!([{
        "code": "noFuncAssign",
        "message": "Reassigning function declaration is not allowed",
        "location": {
          "filename": "no_func_assign",
          "line": 2,
          "col": 2,
        }
      }]),
    )
  }
}
