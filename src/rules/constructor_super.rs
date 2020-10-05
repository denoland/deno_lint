use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{
  Class, ClassMember, Constructor, Expr, ExprOrSuper, Stmt,
};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct ConstructorSuper;

// This rule currently differs from the ESlint implementation
// as there is currently no way of handling code paths in dlint
impl LintRule for ConstructorSuper {
  fn new() -> Box<Self> {
    Box::new(ConstructorSuper)
  }

  fn code(&self) -> &'static str {
    "constructor-super"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = ConstructorSuperVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct ConstructorSuperVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ConstructorSuperVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
  fn check_constructor(&self, constructor: &Constructor, class: &Class) {
    let mut sup = None;
    let mut span = constructor.span;
    if let Some(block_stmt) = &constructor.body {
      span = block_stmt.span;
      for stmt in &block_stmt.stmts {
        if let Stmt::Expr(expr) = stmt {
          if let Expr::Call(call) = &*expr.expr {
            if let ExprOrSuper::Super(s) = &call.callee {
              if sup.is_none() {
                sup = Some(s)
              } else {
                self.context.add_diagnostic(
                  span,
                  "constructor-super",
                  "Constructors of derived classes must call super() only once",
                );
              }
            }
          }
        } else if let Stmt::Return(ret) = stmt {
          // returning value is a substitute of 'super()'.
          if sup.is_none() {
            if ret.arg.is_none() && class.super_class.is_some() {
              self.context.add_diagnostic(
                span,
                "constructor-super",
                "Constructors of derived classes must call super()",
              );
            }
            return;
          }
        }
      }
    }

    if let Some(expr) = &class.super_class {
      if let Expr::Lit(_) = &**expr {
        if constructor.body.is_none()
          || constructor.body.as_ref().unwrap().stmts.is_empty()
        {
          self.context.add_diagnostic(
            span,
            "constructor-super",
            "Classes which inherit from a non constructor must not define a constructor",
          );
        } else {
          self.context.add_diagnostic(
            span,
            "constructor-super",
            "Constructors of classes which inherit from a non constructor must not call super()",
          );
        }
        return;
      }
    }

    if sup.is_some() {
      if class.super_class.is_none() {
        self.context.add_diagnostic(
          span,
          "constructor-super",
          "Constructors of non derived classes must not call super()",
        );
      }
    } else if class.super_class.is_some() {
      self.context.add_diagnostic(
        span,
        "constructor-super",
        "Constructors of derived classes must call super()",
      );
    }
  }
}

impl<'c> Visit for ConstructorSuperVisitor<'c> {
  noop_visit_type!();

  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    for member in &class.body {
      if let ClassMember::Constructor(constructor) = member {
        self.check_constructor(constructor, class);
      }
    }
    swc_ecmascript::visit::visit_class(self, class, parent);
  }
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn constructor_super() {
    assert_lint_ok::<ConstructorSuper>(
      r#"
// non derived classes.
class A { }
class A { constructor() { } }

/*
 * inherit from non constructors.
 * those are valid if we don't define the constructor.
 */
class A extends null { }

// derived classes.
class A extends B { }
class A extends B { constructor() { super(); } }
// class A extends B { constructor() { if (true) { super(); } else { super(); } } }
class A extends (class B {}) { constructor() { super(); } }
class A extends (B = C) { constructor() { super(); } }
class A extends (B || C) { constructor() { super(); } }
class A extends (a ? B : C) { constructor() { super(); } }
class A extends (B, C) { constructor() { super(); } }

// nested.
class A { constructor() { class B extends C { constructor() { super(); } } } }
class A extends B { constructor() { super(); class C extends D { constructor() { super(); } } } }
class A extends B { constructor() { super(); class C { constructor() { } } } }

// multi code path.
// class A extends B { constructor() { a ? super() : super(); } }
// class A extends B { constructor() { if (a) super(); else super(); } }
// class A extends B { constructor() { switch (a) { case 0: super(); break; default: super(); } } }
// class A extends B { constructor() { try {} finally { super(); } } }
// class A extends B { constructor() { if (a) throw Error(); super(); } }

// returning value is a substitute of 'super()'.
class A extends B { constructor() { if (true) return a; super(); } }
class A extends null { constructor() { return a; } }
class A { constructor() { return a; } }

// https://github.com/eslint/eslint/issues/5261
class A extends B { constructor(a) { super(); for (const b of a) { this.a(); } } }

// https://github.com/eslint/eslint/issues/5319
class Foo extends Object { constructor(method) { super(); this.method = method || function() {}; } }
      "#,
    );
    // invalid
    assert_lint_err::<ConstructorSuper>(
      "class A extends null { constructor() { super(); } }",
      37,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends null { constructor() { } }",
      37,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends 100 { constructor() { super(); } }",
      36,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends 'test' { constructor() { super(); } }",
      39,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { for (var a of b) super.foo(); } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { var c = class extends D { constructor() { super(); } } } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { var c = () => super(); } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { var C = class extends D { constructor() { super(); } } } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { super(); super(); } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class A extends B { constructor() { return; super(); } }",
      34,
    );
    assert_lint_err::<ConstructorSuper>(
      "class Foo extends Bar { constructor() { for (a in b) for (c in d); } }",
      38,
    );
    assert_lint_err_on_line::<ConstructorSuper>(
      r#"
class A extends B {
  constructor() {
    class C extends D {
      constructor() {}
    }
    super();
  }
}
        "#,
      5,
      20,
    );
    assert_lint_err_on_line::<ConstructorSuper>(
      r#"
class A extends B {
  constructor() {
    super();
  }
  foo() {
    class C extends D {
      constructor() {}
    }
  }
}
        "#,
      8,
      20,
    );
    assert_lint_err_on_line::<ConstructorSuper>(
      r#"
class A extends B {
  constructor() {
    class C extends null {
      constructor() {
        super();
      }
    }
    super();
  }
}
        "#,
      5,
      20,
    );
    assert_lint_err_on_line::<ConstructorSuper>(
      r#"
class A extends B {
  constructor() {
    class C extends null {
      constructor() {}
    }
    super();
  }
}
        "#,
      5,
      20,
    );
  }
}
