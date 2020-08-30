// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::ClassMethod;
use swc_ecmascript::ast::FnDecl;
use swc_ecmascript::ast::FnExpr;
use swc_ecmascript::ast::Function;
use swc_ecmascript::ast::MethodProp;
use swc_ecmascript::ast::PrivateMethod;
use swc_ecmascript::ast::YieldExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct RequireYield;

impl LintRule for RequireYield {
  fn new() -> Box<Self> {
    Box::new(RequireYield)
  }

  fn code(&self) -> &'static str {
    "require-yield"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = RequireYieldVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct RequireYieldVisitor {
  context: Arc<Context>,
  yield_stack: Vec<u32>,
}

impl RequireYieldVisitor {
  fn new(context: Arc<Context>) -> Self {
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
            "require-yield",
            "Generator function has no `yield`",
          );
        }
      }
    }
  }
}

impl Visit for RequireYieldVisitor {
  noop_visit_type!();

  fn visit_yield_expr(&mut self, _yield_expr: &YieldExpr, _parent: &dyn Node) {
    if let Some(last) = self.yield_stack.last_mut() {
      *last += 1;
    }
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, parent: &dyn Node) {
    self.enter_function(&fn_decl.function);
    swc_ecmascript::visit::visit_fn_decl(self, fn_decl, parent);
    self.exit_function(&fn_decl.function);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, parent: &dyn Node) {
    self.enter_function(&fn_expr.function);
    swc_ecmascript::visit::visit_fn_expr(self, fn_expr, parent);
    self.exit_function(&fn_expr.function);
  }

  fn visit_class_method(
    &mut self,
    class_method: &ClassMethod,
    parent: &dyn Node,
  ) {
    self.enter_function(&class_method.function);
    swc_ecmascript::visit::visit_class_method(self, class_method, parent);
    self.exit_function(&class_method.function);
  }

  fn visit_private_method(
    &mut self,
    private_method: &PrivateMethod,
    parent: &dyn Node,
  ) {
    self.enter_function(&private_method.function);
    swc_ecmascript::visit::visit_private_method(self, private_method, parent);
    self.exit_function(&private_method.function);
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp, parent: &dyn Node) {
    self.enter_function(&method_prop.function);
    swc_ecmascript::visit::visit_method_prop(self, method_prop, parent);
    self.exit_function(&method_prop.function);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn require_yield_ok() {
    assert_lint_ok::<RequireYield>(
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
    );
  }

  #[test]
  fn require_yield() {
    assert_lint_err::<RequireYield>(r#"function* bar() { return "bar"; }"#, 0);
    assert_lint_err::<RequireYield>(
      r#"(function* foo() { return "foo"; })();"#,
      1,
    );
    assert_lint_err::<RequireYield>(
      r#"function* nested() { function* gen() { yield "gen"; } }"#,
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
      vec![(3, 2), (7, 2)],
    );
    assert_lint_err::<RequireYield>(
      r#"const obj = { *foo() { return "foo"; } };"#,
      14,
    );
  }
}
