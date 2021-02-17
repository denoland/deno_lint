// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
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

pub struct RequireYield;

const CODE: &str = "require-yield";
const MESSAGE: &str = "Generator function has no `yield`";

impl LintRule for RequireYield {
  fn new() -> Box<Self> {
    Box::new(RequireYield)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: ProgramRef<'_>,
  ) {
    let mut visitor = RequireYieldVisitor::new(context);
    match program {
        ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
        ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct RequireYieldVisitor<'c> {
  context: &'c mut Context,
  yield_stack: Vec<u32>,
}

impl<'c> RequireYieldVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
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
          self.context.add_diagnostic(function.span, CODE, MESSAGE);
        }
      }
    }
  }
}

impl<'c> Visit for RequireYieldVisitor<'c> {
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

  #[test]
  fn require_yield_valid() {
    assert_lint_ok! {
      RequireYield,
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
    };
  }

  #[test]
  fn require_yield_invalid() {
    assert_lint_err! {
      RequireYield,
      r#"function* bar() { return "bar"; }"#: [{ col: 0, message: MESSAGE }],
      r#"(function* foo() { return "foo"; })();"#: [{ col: 1, message: MESSAGE }],
      r#"function* nested() { function* gen() { yield "gen"; } }"#: [{ col: 0, message: MESSAGE }],
      r#"const obj = { *foo() { return "foo"; } };"#: [{ col: 14, message: MESSAGE }],
      r#"
class Fizz {
  *fizz() {
    return "fizz";
  }

  *#buzz() {
    return "buzz";
  }
}
    "#: [{ line: 3, col: 2, message: MESSAGE }, { line: 7, col: 2, message: MESSAGE }],
    }
  }
}
