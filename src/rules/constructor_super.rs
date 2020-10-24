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

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
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

  fn docs(&self) -> &'static str {
    r#"Verifies the correct usage of constructors and calls to `super()`.

Defined constructors of derived classes (e.g. `class A extends B`) must always call
`super()`.  Classes which extend non-constructors (e.g. `class A extends null`) must
not have a constructor.

### Invalid:
```typescript
class A {}
class Z {
  constructor() {}
}

class B extends Z {
  constructor() {} // missing super() call
} 
class C {
  constructor() {
    super();  // Syntax error
  }
}
class D extends null {
  constructor() {}  // illegal constructor
}
class E extends null {
  constructor() {  // illegal constructor
    super();
  }
}
```

### Valid:
```typescript
class A {}
class B extends A {}
class C extends A {
  constructor() {
    super();
  }
}
class D extends null {}
```
"#
  }
}

enum DiagnosticKind {
  TooManySuper,
  NoSuper,
  UnnecessaryConstructor,
  UnnecessarySuper,
}

impl DiagnosticKind {
  #[cfg(test)]
  fn message_and_hint(&self) -> (&'static str, &'static str) {
    (self.message(), self.hint())
  }

  fn message(&self) -> &'static str {
    match *self {
      DiagnosticKind::TooManySuper => "Constructors of derived classes must call super() only once",
      DiagnosticKind::NoSuper => "Constructors of derived classes must call super()",
      DiagnosticKind::UnnecessaryConstructor => "Classes which inherit from a non constructor must not define a constructor",
      DiagnosticKind::UnnecessarySuper => "Constructors of non derived classes must not call super()",
    }
  }

  fn hint(&self) -> &'static str {
    match *self {
      DiagnosticKind::TooManySuper => "Remove extra calls to super()",
      DiagnosticKind::NoSuper => "Add call to super() in the constructor",
      DiagnosticKind::UnnecessaryConstructor => "Remove constructor",
      DiagnosticKind::UnnecessarySuper => "Remove call to super()",
    }
  }
}

struct ConstructorSuperVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ConstructorSuperVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_constructor(&mut self, constructor: &Constructor, class: &Class) {
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
                let kind = DiagnosticKind::TooManySuper;
                self.context.add_diagnostic_with_hint(
                  span,
                  "constructor-super",
                  kind.message(),
                  kind.hint(),
                );
              }
            }
          }
        } else if let Stmt::Return(ret) = stmt {
          // returning value is a substitute of 'super()'.
          if sup.is_none() {
            if ret.arg.is_none() && class.super_class.is_some() {
              let kind = DiagnosticKind::NoSuper;
              self.context.add_diagnostic_with_hint(
                span,
                "constructor-super",
                kind.message(),
                kind.hint(),
              );
            }
            return;
          }
        }
      }
    }

    if let Some(expr) = &class.super_class {
      if let Expr::Lit(_) = &**expr {
        let kind = DiagnosticKind::UnnecessaryConstructor;
        self.context.add_diagnostic_with_hint(
          span,
          "constructor-super",
          kind.message(),
          kind.hint(),
        );
        return;
      }
    }

    if sup.is_some() {
      if class.super_class.is_none() {
        let kind = DiagnosticKind::UnnecessarySuper;
        self.context.add_diagnostic_with_hint(
          span,
          "constructor-super",
          kind.message(),
          kind.hint(),
        );
      }
    } else if class.super_class.is_some() {
      let kind = DiagnosticKind::TooManySuper;
      self.context.add_diagnostic_with_hint(
        span,
        "constructor-super",
        kind.message(),
        kind.hint(),
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

  #[test]
  fn constructor_super_valid() {
    assert_lint_ok! {
      ConstructorSuper,
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
    };
  }

  #[test]
  fn constructor_super_invalid() {
    let (too_many_super_message, too_many_super_hint) =
      DiagnosticKind::TooManySuper.message_and_hint();
    let (no_super_message, no_super_hint) =
      DiagnosticKind::NoSuper.message_and_hint();
    let (unnecessary_constructor_message, unnecessary_constructor_hint) =
      DiagnosticKind::UnnecessaryConstructor.message_and_hint();
    // TODO(magurotuna): remove this `allow`
    #[allow(unused)]
    let (unnecessary_super_message, unnecessary_super_hint) =
      DiagnosticKind::UnnecessarySuper.message_and_hint();

    assert_lint_err! {
      ConstructorSuper,
      "class A extends null { constructor() { super(); } }": [
        {
          col: 37,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends null { constructor() { } }": [
        {
          col: 37,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends 100 { constructor() { super(); } }": [
        {
          col: 36,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends 'test' { constructor() { super(); } }": [
        {
          col: 39,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends B { constructor() { } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { for (var a of b) super.foo(); } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var c = class extends D { constructor() { super(); } } } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var c = () => super(); } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var C = class extends D { constructor() { super(); } } } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { super(); super(); } }": [
        {
          col: 34,
          message: too_many_super_message,
          hint: too_many_super_hint,
        }
      ],
      "class A extends B { constructor() { return; super(); } }": [
        {
          col: 34,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class Foo extends Bar { constructor() { for (a in b) for (c in d); } }": [
        {
          col: 38,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      r#"
class A extends B {
  constructor() {
    class C extends D {
      constructor() {}
    }
    super();
  }
}
        "#: [
        {
          line: 5,
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
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
        "#: [
        {
          line: 8,
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
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
        "#: [
        {
          line: 5,
          col: 20,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      r#"
class A extends B {
  constructor() {
    class C extends null {
      constructor() {}
    }
    super();
  }
}
        "#: [
        {
          line: 5,
          col: 20,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ]
    };
  }
}
