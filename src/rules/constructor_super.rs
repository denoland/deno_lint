pub struct ConstructorSuper;
use super::Context;
use super::LintRule;
use swc_ecma_ast::{BlockStmt, Class, ClassMember, Expr, ExprOrSuper, Stmt};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

impl LintRule for ConstructorSuper {
  fn new() -> Box<Self> {
    Box::new(ConstructorSuper)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = ConstructorSuperVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct ConstructorSuperVisitor {
  context: Context,
}

impl ConstructorSuperVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
  fn check_constructor_block_stmt(
    &self,
    block_stmt: &BlockStmt,
    class: &Class,
  ) {
    if block_stmt.stmts.iter().any(|stmt| match stmt {
      Stmt::Expr(expr) => match &*expr.expr {
        Expr::Call(call) => match &call.callee {
          ExprOrSuper::Super(_) => true,
          _ => false,
        },
        _ => false,
      },
      _ => false,
    }) {
      if class.super_class.is_none() {
        self.context.add_diagnostic(
          block_stmt.span,
          "constructorSuper",
          "Constructors of non derived classes must not call super()",
        );
      } else if let Some(expr) = &class.super_class {
        if let Expr::Lit(_) = &**expr {
          self.context.add_diagnostic(
            block_stmt.span,
            "constructorSuper",
            "Constructors of classes which which inherit from a non constructor must not call super()",
          );
        }
      }
    } else if class.super_class.is_some() {
      self.context.add_diagnostic(
        block_stmt.span,
        "constructorSuper",
        "Constructors of derived classes must call super()",
      );
    }
  }
}

impl Visit for ConstructorSuperVisitor {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    for member in &class.body {
      if let ClassMember::Constructor(constructor) = member {
        if let Some(block_stmt) = &constructor.body {
          self.check_constructor_block_stmt(block_stmt, class);
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
  fn getter_return() {
    test_lint(
      "constructor_super",
      r#"
class A {
  constructor() {
    super();  // This is a SyntaxError.
  }
}

class B extends A {
  constructor() { }  // Would throw a ReferenceError.
}

// Classes which inherits from a non constructor are always problems.
class C extends null {
  constructor() {
      super();  // Would throw a TypeError.
  }
}

class D extends null {
  constructor() { }  // Would throw a ReferenceError.
}

class E {
  constructor() { }  // Correct
}

class F extends E {
  constructor() {
    super(); // Correct
  }
}
      "#,
      vec![ConstructorSuper::new()],
      json!([{
        "code": "constructorSuper",
        "message": "Constructors of non derived classes must not call super()",
        "location": {
          "filename": "constructor_super",
          "line": 3,
          "col": 16,
        }
      }, {
        "code": "constructorSuper",
        "message": "Constructors of derived classes must call super()",
        "location": {
          "filename": "constructor_super",
          "line": 9,
          "col": 16,
        }
      }, {
        "code": "constructorSuper",
        "message": "Constructors of classes which which inherit from a non constructor must not call super()",
        "location": {
          "filename": "constructor_super",
          "line": 14,
          "col": 16,
        }
      }, {
        "code": "constructorSuper",
        "message": "Constructors of derived classes must call super()",
        "location": {
          "filename": "constructor_super",
          "line": 20,
          "col": 16,
        }
      }]),
    )
  }
}
