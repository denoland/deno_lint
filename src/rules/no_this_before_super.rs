// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::{
  CallExpr, Class, Constructor, ExprOrSuper, Super, ThisExpr,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

pub struct NoThisBeforeSuper;

const CODE: &str = "no-this-before-super";
const MESSAGE: &str = "In the constructor of derived classes, `this` / `super` are not allowed before calling to `super()`.";
const HINT: &str = "Call `super()` before using `this` or `super` keyword.";

impl LintRule for NoThisBeforeSuper {
  fn new() -> Box<Self> {
    Box::new(NoThisBeforeSuper)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
      CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoThisBeforeSuperVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct NoThisBeforeSuperVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoThisBeforeSuperVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoThisBeforeSuperVisitor<'c> {
  noop_visit_type!();

  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut class_visitor =
      ClassVisitor::new(self.context, class.super_class.is_some());
    swc_ecmascript::visit::visit_class(&mut class_visitor, class, parent);
  }
}

struct ClassVisitor<'a> {
  context: &'a mut Context,
  has_super_class: bool,
}

impl<'a> ClassVisitor<'a> {
  fn new(context: &'a mut Context, has_super_class: bool) -> Self {
    Self {
      context,
      has_super_class,
    }
  }
}

impl<'a> Visit for ClassVisitor<'a> {
  noop_visit_type!();

  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut class_visitor =
      ClassVisitor::new(self.context, class.super_class.is_some());
    swc_ecmascript::visit::visit_class(&mut class_visitor, class, parent);
  }

  fn visit_constructor(&mut self, cons: &Constructor, parent: &dyn Node) {
    if self.has_super_class {
      let mut cons_visitor = ConstructorVisitor::new(self.context);
      cons_visitor.visit_constructor(cons, parent);
    } else {
      swc_ecmascript::visit::visit_constructor(self, cons, parent);
    }
  }
}

struct ConstructorVisitor<'a> {
  context: &'a mut Context,
  super_called: bool,
}

impl<'a> ConstructorVisitor<'a> {
  fn new(context: &'a mut Context) -> Self {
    Self {
      context,
      super_called: false,
    }
  }
}

impl<'a> Visit for ConstructorVisitor<'a> {
  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut class_visitor =
      ClassVisitor::new(self.context, class.super_class.is_some());
    swc_ecmascript::visit::visit_class(&mut class_visitor, class, parent);
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
      self.context.add_diagnostic_with_hint(
        this_expr.span,
        CODE,
        MESSAGE,
        HINT,
      );
    }
  }

  fn visit_super(&mut self, sup: &Super, _parent: &dyn Node) {
    if !self.super_called {
      self.context.add_diagnostic_with_hint(
        sup.span,
        CODE,
        MESSAGE,
        HINT,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_this_before_super_valid() {
    assert_lint_ok! {
      NoThisBeforeSuper,
      r#"
class A {
  constructor() {
    this.a = 0;
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
}
      "#,
      r#"
class A extends B {
  foo() {
    this.a = 0;
  }
}
      "#,

      // inline super class
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

      // nested class
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
    };
  }

  #[test]
  fn no_this_before_super_invalid() {
    assert_lint_err! {
      NoThisBeforeSuper,
      r#"
class A extends B {
  constructor() {
    this.a = 0;
    super();
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    this.foo();
    super();
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super.foo();
    super();
  }
}
    "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super(this.foo());
  }
}
    "#: [
        {
          line: 4,
          col: 10,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
    "#: [
        {
          line: 9,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 9,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 10,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 9,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 8,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
