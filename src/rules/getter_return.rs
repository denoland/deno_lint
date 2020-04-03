// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::BlockStmt;
use swc_ecma_ast::Class;
use swc_ecma_ast::ClassMember;
use swc_ecma_ast::GetterProp;
use swc_ecma_ast::MethodKind;
use swc_ecma_ast::Stmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct GetterReturn;

impl LintRule for GetterReturn {
  fn new() -> Box<Self> {
    Box::new(GetterReturn)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = GetterReturnVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct GetterReturnVisitor {
  context: Context,
}

impl GetterReturnVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_block_stmt(&self, block_stmt: &BlockStmt) {
    if !block_stmt.stmts.iter().any(|s| match s {
      Stmt::Return(_) => true,
      _ => false,
    }) {
      self.context.add_diagnostic(
        block_stmt.span,
        "getterReturn",
        "Getter requires a return",
      );
    }
  }
}

impl Visit for GetterReturnVisitor {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    for member in &class.body {
      match member {
        ClassMember::Method(class_method) => {
          if class_method.kind == MethodKind::Getter {
            if let Some(block_stmt) = &class_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        }
        ClassMember::PrivateMethod(private_method) => {
          if private_method.kind == MethodKind::Getter {
            if let Some(block_stmt) = &private_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        }
        _ => {}
      }
    }
  }

  fn visit_getter_prop(
    &mut self,
    getter_prop: &GetterProp,
    _parent: &dyn Node,
  ) {
    if let Some(block_stmt) = &getter_prop.body {
      self.check_block_stmt(block_stmt);
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
      "getter_return",
      r#"
const a = {
  get getter() {}
};

class b {
  get getterA() {}
  private get getterB() {}
}
      "#,
      vec![GetterReturn::new()],
      json!([{
        "code": "getterReturn",
        "message": "Getter requires a return",
        "location": {
          "filename": "getter_return",
          "line": 3,
          "col": 15,
        }
      }, {
        "code": "getterReturn",
        "message": "Getter requires a return",
        "location": {
          "filename": "getter_return",
          "line": 7,
          "col": 16,
        }
      }, {
        "code": "getterReturn",
        "message": "Getter requires a return",
        "location": {
          "filename": "getter_return",
          "line": 8,
          "col": 24,
        }
      }]),
    )
  }
}
