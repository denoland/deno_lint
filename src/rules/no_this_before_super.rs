// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{
  CallExpr, Class, ClassMember, ExprOrSuper, Super, ThisExpr,
};
use swc_ecma_visit::{Node, Visit};

pub struct NoThisBeforeSuper;

impl LintRule for NoThisBeforeSuper {
  fn new() -> Box<Self> {
    Box::new(NoThisBeforeSuper)
  }

  fn code(&self) -> &'static str {
    "no-this-before-super"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoThisBeforeSuperVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoThisBeforeSuperVisitor {
  context: Context,
  super_called: bool,
}

impl NoThisBeforeSuperVisitor {
  pub fn new(context: Context) -> Self {
    Self {
      context,
      super_called: false,
    }
  }

  fn init_on_class(&mut self) {
    self.super_called = false;
  }
}

impl Visit for NoThisBeforeSuperVisitor {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    if class.super_class.is_none() {
      return;
    }

    self.init_on_class();

    let cons = class.body.iter().find_map(|m| match m {
      ClassMember::Constructor(c) => Some(c),
      _ => None,
    });

    if let Some(cons) = cons {
      self.visit_constructor(cons, class);
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    for arg in &call_expr.args {
      self.visit_expr(&*arg.expr, call_expr);
    }

    match call_expr.callee {
      ExprOrSuper::Super(_) => self.super_called = true,
      ExprOrSuper::Expr(ref expr) => self.visit_expr(&**expr, call_expr),
    }
  }

  fn visit_this_expr(&mut self, this_expr: &ThisExpr, _parent: &dyn Node) {
    if !self.super_called {
      self.context.add_diagnostic(
        this_expr.span,
        "no-this-before-super",
        "'this' / 'super' are not allowed before 'super()'.",
      );
    }
  }

  fn visit_super(&mut self, sup: &Super, _parent: &dyn Node) {
    if !self.super_called {
      self.context.add_diagnostic(
        sup.span,
        "no-this-before-super",
        "'this' / 'super' are not allowed before 'super()'.",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_this_before_super_valid() {
    assert_lint_ok::<NoThisBeforeSuper>(
      r#"
class A {
  constructor() {
    this.a = 0;
  }
}
      "#,
    );

    assert_lint_ok::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
}
      "#,
    );

    assert_lint_ok::<NoThisBeforeSuper>(
      r#"
class A extends B {
  foo() {
    this.a = 0;
  }
}
      "#,
    );
  }

  #[test]
  fn no_this_before_super_invalid() {
    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    this.a = 0;
    super();
  }
}
      "#,
      4,
      4,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    this.foo();
    super();
  }
}
      "#,
      4,
      4,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super.foo();
    super();
  }
}
    "#,
      4,
      4,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super(this.foo());
  }
}
    "#,
      4,
      10,
    );
  }
}
