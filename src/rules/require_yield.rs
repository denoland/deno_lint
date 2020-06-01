// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::ClassMethod;
use swc_ecma_ast::FnDecl;
use swc_ecma_ast::FnExpr;
use swc_ecma_ast::Function;
use swc_ecma_ast::MethodProp;
use swc_ecma_ast::PrivateMethod;
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

  fn enter_function(&mut self, function: &Function) {
    if function.is_generator {
      self.yield_stack.push(0);
    }
  }

  fn exit_function(&mut self, function: &Function) {
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

impl Visit for RequireYieldVisitor {
  fn visit_yield_expr(&mut self, _yield_expr: &YieldExpr, _parent: &dyn Node) {
    if let Some(last) = self.yield_stack.last_mut() {
      *last += 1;
    }
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, parent: &dyn Node) {
    self.enter_function(&fn_decl.function);
    swc_ecma_visit::visit_fn_decl(self, fn_decl, parent);
    self.exit_function(&fn_decl.function);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, parent: &dyn Node) {
    self.enter_function(&fn_expr.function);
    swc_ecma_visit::visit_fn_expr(self, fn_expr, parent);
    self.exit_function(&fn_expr.function);
  }

  fn visit_class_method(
    &mut self,
    class_method: &ClassMethod,
    parent: &dyn Node,
  ) {
    self.enter_function(&class_method.function);
    swc_ecma_visit::visit_class_method(self, class_method, parent);
    self.exit_function(&class_method.function);
  }

  fn visit_private_method(
    &mut self,
    private_method: &PrivateMethod,
    parent: &dyn Node,
  ) {
    self.enter_function(&private_method.function);
    swc_ecma_visit::visit_private_method(self, private_method, parent);
    self.exit_function(&private_method.function);
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp, parent: &dyn Node) {
    self.enter_function(&method_prop.function);
    swc_ecma_visit::visit_method_prop(self, method_prop, parent);
    self.exit_function(&method_prop.function);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;
  use serde_json::json;

  #[test]
  fn require_yield_ok() {
    assert_lint_ok::<RequireYield>(vec![
      r#"
function foo() {}
function* bar() { 
  yield "bar";
}
function* emptyBar() {}

class Fizz {
  *fizz() {
    yield "fizz";
  }

  *#buzz() {
    yield "buzz";
  }
}

const obj = {
  *foo() {
    yield "foo";
  }
};
      "#,
    ]);
  }

  #[test]
  fn require_yield() {
    assert_lint_err_on_line::<RequireYield>(
      r#"
function* bar() { 
  return "bar";
}
    "#,
      "requireYield",
      2,
      0,
    );
    assert_lint_err_on_line::<RequireYield>(
      r#"
(function* foo() {
  return "foo";
})();
    "#,
      "requireYield",
      2,
      1,
    );
    assert_lint_err_on_line::<RequireYield>(
      r#"
function* nested() {
  function* gen() {
    yield "gen";
  }
}
    "#,
      "requireYield",
      2,
      0,
    );
    assert_lint_err_on_line_n::<RequireYield>(
      r#"
class Fizz {
  *fizz() {
    return "fizz";
  }

  *#buzz() {
    return "buzz";
  }
}
    "#,
      vec![("requireYield", 3, 2), ("requireYield", 7, 2)],
    );
    assert_lint_err_on_line::<RequireYield>(
      r#"
const obj = {
  *foo() {
    return "foo";
  }
};
    "#,
      "requireYield",
      3,
      2,
    );
  }
}
