// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::BlockStmt;
use swc_ecma_ast::Class;
use swc_ecma_ast::ClassMember;
use swc_ecma_ast::MethodKind;
use swc_ecma_ast::SetterProp;
use swc_ecma_ast::Stmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoSetterReturn;

impl LintRule for NoSetterReturn {
  fn new() -> Box<Self> {
    Box::new(NoSetterReturn)
  }

  fn code(&self) -> &'static str {
    "noSetterReturn"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoSetterReturnVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoSetterReturnVisitor {
  context: Context,
}

impl NoSetterReturnVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_block_stmt(&self, block_stmt: &BlockStmt) {
    for stmt in &block_stmt.stmts {
      if let Stmt::Return(return_stmt) = stmt {
        if return_stmt.arg.is_some() {
          self.context.add_diagnostic(
            return_stmt.span,
            "noSetterReturn",
            "Setter cannot return a value",
          );
        }
      }
    }
  }
}

impl Visit for NoSetterReturnVisitor {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    for member in &class.body {
      match member {
        ClassMember::Method(class_method) => {
          if class_method.kind == MethodKind::Setter {
            if let Some(block_stmt) = &class_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        }
        ClassMember::PrivateMethod(private_method) => {
          if private_method.kind == MethodKind::Setter {
            if let Some(block_stmt) = &private_method.function.body {
              self.check_block_stmt(block_stmt);
            }
          }
        }
        _ => {}
      }
    }
  }

  fn visit_setter_prop(
    &mut self,
    setter_prop: &SetterProp,
    _parent: &dyn Node,
  ) {
    if let Some(block_stmt) = &setter_prop.body {
      self.check_block_stmt(block_stmt);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn setter_return() {
    assert_lint_err::<NoSetterReturn>(
      r#"const a = { set setter(a) { return "something"; } };"#,
      "noSetterReturn",
      28,
    );
    assert_lint_err_on_line_n::<NoSetterReturn>(
      r#"
class b {
  set setterA(a) {
    return "something";
  }
  private set setterB(a) {
    return "something";
  }
}
      "#,
      vec![("noSetterReturn", 4, 4), ("noSetterReturn", 7, 4)],
    );
  }
}
