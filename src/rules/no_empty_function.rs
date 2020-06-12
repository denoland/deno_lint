// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::FnDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyFunction;

impl LintRule for NoEmptyFunction {
  fn new() -> Box<Self> {
    Box::new(NoEmptyFunction)
  }

  fn code(&self) -> &'static str {
    "no-empty-function"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEmptyFunctionVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoEmptyFunctionVisitor {
  context: Context,
}

impl NoEmptyFunctionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyFunctionVisitor {
  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _parent: &dyn Node) {
    let body = fn_decl.function.body.as_ref();
    if body.is_none() || body.unwrap().stmts.is_empty() {
      self.context.add_diagnostic(
        fn_decl.function.span,
        "no-empty-function",
        "Empty functions are not allowed",
      )
    } else {
      for stmt in &fn_decl.function.body {
        self.visit_block_stmt(stmt, _parent);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_function_test() {
    assert_lint_err::<NoEmptyFunction>(
      "function emptyFunctionWithBody() { }",
      0,
    );
    assert_lint_err::<NoEmptyFunction>(
      "function emptyFunctionWithoutBody();",
      0,
    );
  }

  #[test]
  fn no_empty_function_nested_test() {
    assert_lint_err::<NoEmptyFunction>(
      "function parentFunction() { function childFunction(); }",
      28,
    );
  }
}
