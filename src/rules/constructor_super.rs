pub struct ConstructorSuper;

use super::Context;
use super::LintRule;
use swc_ecma_ast::{Class, ClassMember, Constructor, Expr, ExprOrSuper, Stmt};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

// This rule currently differs from the ESlint implementation
// as there is currently no way of handling code paths in dlint
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
                  "constructorSuper",
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
                "constructorSuper",
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
            "constructorSuper",
            "Classes which inherit from a non constructor must not define a constructor",
          );
        } else {
          self.context.add_diagnostic(
            span,
            "constructorSuper",
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
          "constructorSuper",
          "Constructors of non derived classes must not call super()",
        );
      }
    } else if class.super_class.is_some() {
      self.context.add_diagnostic(
        span,
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
        self.check_constructor(constructor, class);
      }
    }
  }
}

// tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn getter_return() {
    // valid
    test_lint(
      "constructor_super",
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
      vec![ConstructorSuper::new()],
      json!([]),
    );
    // invalid
    test_lint(
      "constructor_super",
      r#"
// inherit from non constructors.
class A extends null { constructor() { super(); } }
class A extends null { constructor() { } }
class A extends 100 { constructor() { super(); } }
class A extends 'test' { constructor() { super(); } }
// derived classes.
class A extends B { constructor() { } }
class A extends B { constructor() { for (var a of b) super.foo(); } }
// nested execution scope.
class A extends B { constructor() { class C extends D { constructor() { super(); } } } }
class A extends B { constructor() { var c = class extends D { constructor() { super(); } } } }
class A extends B { constructor() { var c = () => super(); } }
class A extends B { constructor() { class C extends D { constructor() { super(); } } } }
class A extends B { constructor() { var C = class extends D { constructor() { super(); } } } }
// ? class A extends B { constructor() { super(); class C extends D { constructor() { } } } }
// ? class A extends B { constructor() { super(); var C = class extends D { constructor() { } } } }
// lacked in some code path.
// class A extends B { constructor() { if (a) super(); } }
// class A extends B { constructor() { if (a); else super(); } }
// class A extends B { constructor() { a && super(); } }
// class A extends B { constructor() { switch (a) { case 0: super(); } } }
// class A extends B { constructor() { switch (a) { case 0: break; default: super(); } } }
// class A extends B { constructor() { try { super(); } catch (err) {} } }
// class A extends B { constructor() { try { a; } catch (err) { super(); } } }
// class A extends B { constructor() { if (a) return; super(); } }
// duplicate.
class A extends B { constructor() { super(); super(); } }
// class A extends B { constructor() { super() || super(); } }
// class A extends B { constructor() { if (a) super(); super(); } }
// class A extends B { constructor() { switch (a) { case 0: super(); default: super(); } } }
// class A extends B { constructor(a) { while (a) super(); } }
// ignores `super()` on unreachable paths.
class A extends B { constructor() { return; super(); } }
// https://github.com/eslint/eslint/issues/8248
class Foo extends Bar {
                constructor() {
                    for (a in b) for (c in d);
                }
            }
      "#,
      vec![ConstructorSuper::new()],
      json!([
        {
          "code": "constructorSuper",
          "location": {
            "col": 37,
            "filename": "constructor_super",
            "line": 3
          },
          "message": "Constructors of classes which inherit from a non constructor must not call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 37,
            "filename": "constructor_super",
            "line": 4
          },
          "message": "Classes which inherit from a non constructor must not define a constructor"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 36,
            "filename": "constructor_super",
            "line": 5
          },
          "message": "Constructors of classes which inherit from a non constructor must not call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 39,
            "filename": "constructor_super",
            "line": 6
          },
          "message": "Constructors of classes which inherit from a non constructor must not call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 8
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 9
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 11
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 12
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 13
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 14
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 15
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 28
          },
          "message": "Constructors of derived classes must call super() only once"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 34,
            "filename": "constructor_super",
            "line": 34
          },
          "message": "Constructors of derived classes must call super()"
        },
        {
          "code": "constructorSuper",
          "location": {
            "col": 30,
            "filename": "constructor_super",
            "line": 37
          },
          "message": "Constructors of derived classes must call super()"
        }
      ]),
    );
  }
}
