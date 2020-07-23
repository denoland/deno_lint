// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::AssignExpr;
use crate::swc_ecma_ast::Pat;
use crate::swc_ecma_ast::PatOrExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct NoFuncAssign;

impl LintRule for NoFuncAssign {
  fn new() -> Box<Self> {
    Box::new(NoFuncAssign)
  }

  fn code(&self) -> &'static str {
    "no-func-assign"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoFuncAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoFuncAssignVisitor {
  context: Arc<Context>,
}

impl NoFuncAssignVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoFuncAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let name = match &assign_expr.left {
      PatOrExpr::Expr(_) => return,
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => ident.sym.as_ref(),
        _ => return,
      },
    };

    let scope = self.context.root_scope.get_scope_for_span(assign_expr.span);
    let bindings = scope.get_bindings();
    if let Some(binding) = bindings.iter().find(|b| b.name == name)
    {
      if binding.kind == BindingKind::Function {
        self.context.add_diagnostic(
          assign_expr.span,
          "no-func-assign",
          "Reassigning function declaration is not allowed",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line;

  #[test]
  fn no_func_assign() {
    assert_lint_err_on_line::<NoFuncAssign>(
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
      11,
      0,
    );
  }
}
