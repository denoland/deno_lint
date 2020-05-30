// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::FnDecl;
use swc_ecma_ast::FnExpr;
use swc_ecma_ast::Function;
use swc_ecma_ast::YieldExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct RequireYield;

impl LintRule for RequireYield {
  fn new() -> Box<Self> {
    Box::new(RequireYield)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = RequireYieldVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct RequireYieldVisitor {
  context: Context,
  yield_stack: Vec<u32>,
}

impl RequireYieldVisitor {
  pub fn new(context: Context) -> Self {
    Self {
      context,
      yield_stack: vec![],
    }
  }

  fn check_function(&mut self, function: &Function) {
    if function.is_generator {
      let yield_count = self.yield_stack.pop().unwrap();

      // Verify that `yield` was called only if function body
      // is non-empty
      if let Some(body) = &function.body {
        if !body.stmts.is_empty() && yield_count == 0 {
          self.context.add_diagnostic(
            function.span,
            "requireYield",
            "Generator function has no `yield`",
          );
        }
      }
    }
  }
}

// TODO(bartlomieju): class methods and fn expr in object should be handled as well
impl Visit for RequireYieldVisitor {
  fn visit_yield_expr(&mut self, _yield_expr: &YieldExpr, _parent: &dyn Node) {
    if let Some(last) = self.yield_stack.last_mut() {
      *last += 1;
    }
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, parent: &dyn Node) {
    if fn_decl.function.is_generator {
      self.yield_stack.push(0);
    }

    swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);

    self.check_function(&fn_decl.function);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, parent: &dyn Node) {
    if fn_expr.function.is_generator {
      self.yield_stack.push(0);
    }

    swc_ecma_visit::visit_fn_expr(self, fn_expr, parent);

    self.check_function(&fn_expr.function);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn require_yield_ok() {
    test_lint(
      "require_yield",
      r#"
function foo() {}
function* bar() { 
  yield "bar";
}
function* emptyBar() {}
      "#,
      vec![RequireYield::new()],
      json!([]),
    )
  }

  #[test]
  fn require_yield() {
    test_lint(
      "require_yield",
      r#"
function* bar() { 
  return "bar";
}

(function* foo() {
  return "foo";
})();

function* nested() {
  function* gen() {
    yield "gen";
  }
}
      "#,
      vec![RequireYield::new()],
      json!([{
        "code": "requireYield",
        "message": "Generator function has no `yield`",
        "location": {
          "filename": "require_yield",
          "line": 2,
          "col": 0,
        }
      },{
        "code": "requireYield",
        "message": "Generator function has no `yield`",
        "location": {
          "filename": "require_yield",
          "line": 6,
          "col": 1,
        }
      },
      {
        "code": "requireYield",
        "message": "Generator function has no `yield`",
        "location": {
          "filename": "require_yield",
          "line": 10,
          "col": 0,
        }
      }]),
    )
  }
}
