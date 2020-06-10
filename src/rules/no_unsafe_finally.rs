// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecma_ast::Module;
use swc_ecma_ast::Stmt::{Break, Continue, Return, Throw};
use swc_ecma_ast::TryStmt;
use swc_ecma_visit::{Node, Visit};

pub struct NoUnsafeFinally;

impl LintRule for NoUnsafeFinally {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeFinally)
  }

  fn code(&self) -> &'static str {
    "no-unsafe-finally"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoUnsafeFinallyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoUnsafeFinallyVisitor {
  context: Context,
}

impl NoUnsafeFinallyVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoUnsafeFinallyVisitor {
  fn visit_try_stmt(&mut self, try_stmt: &TryStmt, _parent: &dyn Node) {
    if let Some(finally_block) = &try_stmt.finalizer {
      // Convenience function for providing different diagnostic message
      // depending on statement type
      let add_diagnostic = |stmt_type: &str| {
        self.context.add_diagnostic(
          finally_block.span,
          "no-unsafe-finally",
          format!("Unsafe usage of {}Statement", stmt_type).as_str(),
        );
      };

      for stmt in &finally_block.stmts {
        match stmt {
          Break(_) => add_diagnostic("Break"),
          Continue(_) => add_diagnostic("Continue"),
          Return(_) => add_diagnostic("Return"),
          Throw(_) => add_diagnostic("Throw"),
          _ => {}
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_when_there_are_no_disallowed_keywords_in_the_finally_block() {
    assert_lint_ok::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    console.log("hola!");
  }
};
     "#,
    );
  }

  #[test]
  fn it_passes_for_a_return_within_a_function_in_a_finally_block() {
    assert_lint_ok::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    let a = function() {
      return "hola!";
    }
  }
};
     "#,
    );
  }

  #[test]
  fn it_passes_for_a_break_within_a_switch_in_a_finally_block() {
    assert_lint_ok::<NoUnsafeFinally>(
      r#"
let foo = function(a) {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    switch(a) {
      case 1: {
        console.log("hola!")
        break;
      }
    }
  }
};
      "#,
    );
  }

  #[test]
  fn it_fails_for_a_break_in_a_finally_block() {
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    break;
  }
};
     "#,
      7,
      12,
    );
  }

  #[test]
  fn it_fails_for_a_continue_in_a_finally_block() {
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    continue;
  }
};
     "#,
      7,
      12,
    );
  }

  #[test]
  fn it_fails_for_a_return_in_a_finally_block() {
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    return 3;
  }
};
          "#,
      7,
      12,
    );
  }

  #[test]
  fn it_fails_for_a_throw_in_a_finally_block() {
    assert_lint_err_on_line::<NoUnsafeFinally>(
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    throw new Error;
  }
};
     "#,
      7,
      12,
    );
  }
}
