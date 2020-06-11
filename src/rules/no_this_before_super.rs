// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{
  CallExpr, Class, Constructor, ExprOrSuper, Super, ThisExpr,
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
  has_super_class: bool,
}

impl NoThisBeforeSuperVisitor {
  fn new(context: Context) -> Self {
    Self {
      context,
      has_super_class: false,
    }
  }
}

impl Visit for NoThisBeforeSuperVisitor {
  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let tmp = self.has_super_class;

    self.has_super_class = class.super_class.is_some();
    swc_ecma_visit::visit_class(self, class, parent);

    self.has_super_class = tmp;
  }

  fn visit_constructor(&mut self, cons: &Constructor, parent: &dyn Node) {
    if self.has_super_class {
      let mut cons_visitor = ConstructorVisitor::new(&self.context);
      cons_visitor.visit_constructor(cons, parent);
    } else {
      swc_ecma_visit::visit_constructor(self, cons, parent);
    }
  }
}

struct ConstructorVisitor<'a> {
  context: &'a Context,
  super_called: bool,
}

impl<'a> ConstructorVisitor<'a> {
  fn new(context: &'a Context) -> Self {
    Self {
      context,
      super_called: false,
    }
  }
}

impl<'a> Visit for ConstructorVisitor<'a> {
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

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super();
  }
}
class C extends D {
  constructor() {
    this.c = 42;
    super();
  }
}
    "#,
      9,
      4,
    );
  }

  #[test]
  fn no_this_before_super_inline_super_class() {
    assert_lint_ok::<NoThisBeforeSuper>(
      r#"
class A extends class extends B {
  constructor() {
    super();
    this.a = 0;
  }
} {
    constructor() {
      super();
      this.a = 0;
    }
}
      "#,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends class extends B {
  constructor() {
    this.a = 0;
    super();
  }
} {
    constructor() {
      super();
      this.a = 0;
    }
}
      "#,
      4,
      4,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends class extends B {
  constructor() {
    super();
    this.a = 0;
  }
} {
    constructor() {
      this.a = 0;
      super();
    }
}
      "#,
      9,
      6,
    );
  }

  #[test]
  fn no_this_before_super_nested_class() {
    assert_lint_ok::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        super();
        this.c = 1;
      }
    }
  }
}
      "#,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#,
      10,
      8,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    this.a = 0;
    super();
  }
  foo() {
    class C extends D {
      constructor() {
        super();
        this.c = 1;
      }
    }
  }
}
      "#,
      4,
      4,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A {
  constructor() {
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#,
      9,
      8,
    );

    assert_lint_err_on_line::<NoThisBeforeSuper>(
      r#"
class A extends B {
  constructor() {
    this.a = 0;
  }
  foo() {
    class C {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#,
      4,
      4,
    );
  }
}
