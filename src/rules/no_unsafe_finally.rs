use super::{Context, LintRule};
use swc_ecma_ast::{Module};
use swc_ecma_ast::TryStmt;
use swc_ecma_ast::Stmt::{Break, Continue,Return, Throw};
use swc_ecma_visit::{Node, Visit};

pub struct NoUnsafeFinally;

impl LintRule for NoUnsafeFinally {
  fn new() -> Box<Self> {
    Box::new(NoUnsafeFinally)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoUnsafeFinallyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoUnsafeFinallyVisitor {
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
          "noUnsafeFinally",
          format!("Unsafe usage of {}", stmt_type).as_str(),
        );
      };

      for stmt in &finally_block.stmts {
        match stmt {
          Break(_) => add_diagnostic("BreakStatement"),
          Continue(_) => add_diagnostic("ContinueStatement"),
          Return(_) => add_diagnostic("ReturnStatement"),
          Throw(_) => add_diagnostic("ThrowStatement"),
          _ => {},
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn it_passes_when_there_are_no_disallowed_keywords_in_the_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_for_a_return_within_a_function_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([]),
    )
  }

  #[test]
  fn it_passes_for_a_break_within_a_switch_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([]),
    )
  }

  #[test]
  fn it_fails_for_a_break_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([{
        "code": "noUnsafeFinally",
        "message": "Unsafe usage of BreakStatement",
        "location": {
          "filename": "no_unsafe_finally",
          "line": 7,
          "col": 12,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_a_continue_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([{
        "code": "noUnsafeFinally",
        "message": "Unsafe usage of ContinueStatement",
        "location": {
          "filename": "no_unsafe_finally",
          "line": 7,
          "col": 12,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_a_return_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([{
        "code": "noUnsafeFinally",
        "message": "Unsafe usage of ReturnStatement",
        "location": {
          "filename": "no_unsafe_finally",
          "line": 7,
          "col": 12,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_for_a_throw_in_a_finally_block() {
    test_lint(
      "no_unsafe_finally",
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
      vec![NoUnsafeFinally::new()],
      json!([{
        "code": "noUnsafeFinally",
        "message": "Unsafe usage of ThrowStatement",
        "location": {
          "filename": "no_unsafe_finally",
          "line": 7,
          "col": 12,
        }
      }]),
    )
  }
}