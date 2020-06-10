// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_common::Span;
use swc_ecma_ast::Expr::Assign;
use swc_ecma_ast::Module;
use swc_ecma_ast::Stmt::{self, DoWhile, For, If, While};
use swc_ecma_visit::{Node, Visit};

pub struct NoCondAssign;

impl LintRule for NoCondAssign {
  fn new() -> Box<Self> {
    Box::new(NoCondAssign)
  }

  fn code(&self) -> &'static str {
    "no-cond-assign"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoCondAssignVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoCondAssignVisitor {
  context: Context,
}

impl NoCondAssignVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-cond-assign",
      "Expected a conditional expression and instead saw an assignment",
    );
  }
}

impl Visit for NoCondAssignVisitor {
  fn visit_stmt(&mut self, stmt: &Stmt, _parent: &dyn Node) {
    match stmt {
      If(if_stmt) => {
        if let Assign(assign) = &*if_stmt.test {
          self.add_diagnostic(assign.span);
        }
      }
      While(while_stmt) => {
        if let Assign(assign) = &*while_stmt.test {
          self.add_diagnostic(assign.span);
        }
      }
      DoWhile(do_while) => {
        if let Assign(assign) = &*do_while.test {
          self.add_diagnostic(assign.span);
        }
      }
      For(for_stmt) => {
        if let Some(Assign(assign)) = for_stmt.test.as_deref() {
          self.add_diagnostic(assign.span);
        }
      }
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_using_equality_operator() {
    assert_lint_ok::<NoCondAssign>("if (x === 0) { }");
  }

  #[test]
  fn it_passes_with_bracketed_assignment() {
    assert_lint_ok::<NoCondAssign>("if ((x = y)) { }");
  }

  #[test]
  fn it_fails_using_assignment_in_if_stmt() {
    assert_lint_err::<NoCondAssign>("if (x = 0) { }", 4);
  }

  #[test]
  fn it_fails_using_assignment_in_while_stmt() {
    assert_lint_err::<NoCondAssign>("while (x = 0) { }", 7);
  }

  #[test]
  fn it_fails_using_assignment_in_do_while_stmt() {
    assert_lint_err::<NoCondAssign>("do { } while (x = 0);", 14);
  }

  #[test]
  fn it_fails_using_assignment_in_for_stmt() {
    assert_lint_err::<NoCondAssign>("for (let i = 0; i = 10; i++) { }", 16);
  }
}
