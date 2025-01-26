// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::tags::{self, Tags};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::ClassMethod;
use deno_ast::swc::ast::FnDecl;
use deno_ast::swc::ast::FnExpr;
use deno_ast::swc::ast::Function;
use deno_ast::swc::ast::MethodProp;
use deno_ast::swc::ast::PrivateMethod;
use deno_ast::swc::ast::YieldExpr;
use deno_ast::swc::visit::Visit;
use deno_ast::swc::visit::{noop_visit_type, VisitWith};
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct RequireYield;

const CODE: &str = "require-yield";
const MESSAGE: &str = "Generator function has no `yield`";

impl LintRule for RequireYield {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = RequireYieldVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
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
          self.context.add_diagnostic(function.range(), CODE, MESSAGE);
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
    fn_decl.visit_children_with(self);
    self.exit_function(&fn_decl.function);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    self.enter_function(&fn_expr.function);
    fn_expr.visit_children_with(self);
    self.exit_function(&fn_expr.function);
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod) {
    self.enter_function(&class_method.function);
    class_method.visit_children_with(self);
    self.exit_function(&class_method.function);
  }

  fn visit_private_method(&mut self, private_method: &PrivateMethod) {
    self.enter_function(&private_method.function);
    private_method.visit_children_with(self);
    self.exit_function(&private_method.function);
  }

  fn visit_method_prop(&mut self, method_prop: &MethodProp) {
    self.enter_function(&method_prop.function);
    method_prop.visit_children_with(self);
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
