// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

use std::sync::Arc;

pub struct NoClassAssign;

impl LintRule for NoClassAssign {
  fn new() -> Box<Self> {
    Box::new(NoClassAssign)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }
  fn code(&self) -> &'static str {
    "no-class-assign"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoClassAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoClassAssignVisitor {
  context: Arc<Context>,
}

impl NoClassAssignVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoClassAssignVisitor {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    assign_expr.visit_children_with(self);

    let ids = find_lhs_ids(&assign_expr.left);
    for id in ids {
      let var = self.context.scope.var(&id);
      if let Some(var) = var {
        if let BindingKind::Class = var.kind() {
          self.context.add_diagnostic(
            assign_expr.span,
            "no-class-assign",
            "Reassigning class declaration is not allowed",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-class-assign.js
  // MIT Licensed.

  #[test]
  fn no_class_assign_valid() {
    assert_lint_ok_n::<NoClassAssign>(vec![
      r#"class A {}"#,
      r#"class A {} foo(A);"#,
      r#"let A = class A {}; foo(A);"#,
      r#"
class A {
  foo(A) {
    A = "foobar";
  }
}
"#,
      r#"
class A {
  foo() {
    let A;
    A = "bar";
  }
}
"#,
      r#"
let A = class {
  b() {
    A = 0;
  }
}
"#,
      r#"
let A, B;
A = class {
  b() {
    B = 0;
  }
}
"#,
      r#"let x = 0; x = 1;"#,
      r#"var x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"function x() {} x = 1;"#,
      r#"function foo(x) { x = 1; }"#,
      r#"try {} catch (x) { x = 1; }"#,
    ]);
  }

  #[test]
  fn no_class_assign_invalid() {
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
class A {}
A = 0;
      "#,
      3,
      0,
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
class A {}
({A} = 0);
      "#,
      3,
      1,
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
class A {}
({b: A = 0} = {});
      "#,
      3,
      1,
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
A = 0;
class A {}
      "#,
      2,
      0,
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
class A {
  foo() {
    A = 0;
  }
}
      "#,
      4,
      4,
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
let A = class A {
  foo() {
    A = 0;
  }
}
      "#,
      4,
      4,
    );
    assert_lint_err_on_line_n::<NoClassAssign>(
      r#"
class A {}
A = 10;
A = 20;
      "#,
      vec![(3, 0), (4, 0)],
    );
    assert_lint_err_on_line::<NoClassAssign>(
      r#"
let A;
A = class {
  foo() {
    class B {}
    B = 0;
  }
}
      "#,
      6,
      4,
    );
  }
}
