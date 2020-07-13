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

pub struct NoFuncAssign;

impl LintRule for NoFuncAssign {
  fn new() -> Box<Self> {
    Box::new(NoFuncAssign)
  }

  fn code(&self) -> &'static str {
    "no-func-assign"
  }

  fn lint_module(&self, context: Context, module: &swc_ecma_ast::Module) {
    let mut visitor = NoFuncAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoFuncAssignVisitor {
  context: Context,
}

impl NoFuncAssignVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
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

    let scope = self
      .context
      .scope_manager
      .get_scope_for_span(assign_expr.span);
    if let Some(binding) = self.context.scope_manager.get_binding(scope, &ident)
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
