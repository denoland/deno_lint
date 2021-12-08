// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::ProgramRef;
use deno_ast::swc::ast::ClassMethod;
use deno_ast::swc::ast::FnDecl;
use deno_ast::swc::ast::FnExpr;
use deno_ast::swc::ast::Function;
use deno_ast::swc::ast::MethodProp;
use deno_ast::swc::ast::PrivateMethod;
use deno_ast::swc::ast::YieldExpr;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Visit;
use std::sync::Arc;

#[derive(Debug)]
pub struct RequireYield;

const CODE: &str = "require-yield";
const MESSAGE: &str = "Generator function has no `yield`";

impl LintRule for RequireYield {
  fn new() -> Arc<Self> {
    Arc::new(RequireYield)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = RequireYieldVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/require_yield.md")
  }
}

struct RequireYieldVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  yield_stack: Vec<u32>,
}

impl<'c, 'view> RequireYieldVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
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

impl<'c, 'view> Visit for RequireYieldVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_yield_expr(&mut self, _yield_expr: &YieldExpr) {
    if let Some(last) = self.yield_stack.last_mut() {
      *last += 1;
    }
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
    self.enter_function(&fn_decl.function);
    deno_ast::swc::visit::visit_fn_decl(self, fn_decl);
    self.exit_function(&fn_decl.function);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    self.enter_function(&fn_expr.function);
    deno_ast::swc::visit::visit_fn_expr(self, fn_expr);
    self.exit_function(&fn_expr.function);
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod) {
    self.enter_function(&class_method.function);
    deno_ast::swc::visit::visit_class_method(self, class_method);
    self.exit_function(&class_method.function);
  }

  fn visit_private_method(&mut self, private_method: &PrivateMethod) {
    self.enter_function(&private_method.function);
    deno_ast::swc::visit::visit_private_method(self, private_method);
    self.exit_function(&private_method.function);
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp) {
    self.enter_function(&method_prop.function);
    deno_ast::swc::visit::visit_method_prop(self, method_prop);
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
