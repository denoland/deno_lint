use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{
  Class, ClassMember, Constructor, Expr, ExprOrSuper, ReturnStmt, Stmt,
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

fn inherits_from_non_constructor(class: &Class) -> bool {
  if let Some(expr) = &class.super_class {
    if let Expr::Lit(_) = &**expr {
      return true;
    }
  }
  false
}

fn super_call_spans(constructor: &Constructor) -> Vec<Span> {
  if let Some(block_stmt) = &constructor.body {
    block_stmt
      .stmts
      .iter()
      .filter_map(|stmt| extract_super_span(stmt))
      .collect()
  } else {
    vec![]
  }
}

fn extract_super_span(stmt: &Stmt) -> Option<Span> {
  if let Stmt::Expr(expr) = stmt {
    if let Expr::Call(call) = &*expr.expr {
      if matches!(&call.callee, ExprOrSuper::Super(_)) {
        return Some(call.span);
      }
    }
  }
  None
}

fn return_before_super(constructor: &Constructor) -> Option<&ReturnStmt> {
  if let Some(block_stmt) = &constructor.body {
    for stmt in &block_stmt.stmts {
      if extract_super_span(stmt).is_some() {
        return None;
      }

      if let Stmt::Return(ret) = stmt {
        return Some(ret);
      }
    }
  }
  None
}

struct ConstructorSuperVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ConstructorSuperVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_constructor(&mut self, constructor: &Constructor, class: &Class) {
    // returning value is a substitute of 'super()'.
    if let Some(ret) = return_before_super(constructor) {
      if ret.arg.is_none() && class.super_class.is_some() {
        let kind = DiagnosticKind::NoSuper;
        self.context.add_diagnostic_with_hint(
          constructor.span,
          "constructor-super",
          kind.message(),
          kind.hint(),
        );
      }
      return;
    }

    if inherits_from_non_constructor(class) {
      let kind = DiagnosticKind::UnnecessaryConstructor;
      self.context.add_diagnostic_with_hint(
        constructor.span,
        "constructor-super",
        kind.message(),
        kind.hint(),
      );
      return;
    }

    let super_calls = super_call_spans(constructor);

    // in case where there are more than one `super()` calls.
    for exceeded_super_span in super_calls.iter().skip(1) {
      let kind = DiagnosticKind::TooManySuper;
      self.context.add_diagnostic_with_hint(
        *exceeded_super_span,
        "constructor-super",
        kind.message(),
        kind.hint(),
      );
    }

    match (super_calls.is_empty(), class.super_class.is_some()) {
      (true, true) => {
        let kind = DiagnosticKind::NoSuper;
        self.context.add_diagnostic_with_hint(
          constructor.span,
          "constructor-super",
          kind.message(),
          kind.hint(),
        );
      }
      (false, false) => {
        let kind = DiagnosticKind::UnnecessarySuper;
        self.context.add_diagnostic_with_hint(
          super_calls[0],
          "constructor-super",
          kind.message(),
          kind.hint(),
        );
      }
      _ => {}
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
      // non derived classes.
      "class A { }",
      "class A { constructor() { } }",

      // inherit from non constructors.
      // those are valid if we don't define the constructor.
      "class A extends null { }",

      // derived classes.
      "class A extends B { }",
      "class A extends B { constructor() { super(); } }",

      // TODO(magurotuna): control flow analysis is required to handle these cases
      // "class A extends B { constructor() { if (true) { super(); } else { super(); } } }",
      // "class A extends B { constructor() { a ? super() : super(); } }",
      // "class A extends B { constructor() { if (a) super(); else super(); } }",
      // "class A extends B { constructor() { switch (a) { case 0: super(); break; default: super(); } } }",
      // "class A extends B { constructor() { try {} finally { super(); } } }",
      // "class A extends B { constructor() { if (a) throw Error(); super(); } }",

      // derived classes.
      "class A extends (class B {}) { constructor() { super(); } }",
      "class A extends (B = C) { constructor() { super(); } }",
      "class A extends (B || C) { constructor() { super(); } }",
      "class A extends (a ? B : C) { constructor() { super(); } }",
      "class A extends (B, C) { constructor() { super(); } }",

      // nested.
      "class A { constructor() { class B extends C { constructor() { super(); } } } }",
      "class A extends B { constructor() { super(); class C extends D { constructor() { super(); } } } }",
      "class A extends B { constructor() { super(); class C { constructor() { } } } }",

      // returning value is a substitute of 'super()'.
      "class A extends B { constructor() { if (true) return a; super(); } }",
      "class A extends null { constructor() { return a; } }",
      "class A { constructor() { return a; } }",

      // https://github.com/eslint/eslint/issues/5261
      "class A extends B { constructor(a) { super(); for (const b of a) { this.a(); } } }",

      // https://github.com/eslint/eslint/issues/5319
      "class Foo extends Object { constructor(method) { super(); this.method = method || function() {}; } }",
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
    let (unnecessary_super_message, unnecessary_super_hint) =
      DiagnosticKind::UnnecessarySuper.message_and_hint();

    assert_lint_err! {
      ConstructorSuper,
      "class A { constructor() { super(); } }": [
        {
          col: 26,
          message: unnecessary_super_message,
          hint: unnecessary_super_hint,
        }
      ],
      "class A extends null { constructor() { super(); } }": [
        {
          col: 23,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends null { constructor() { } }": [
        {
          col: 23,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends 1000 { constructor() { super(); } }": [
        {
          col: 23,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends 'ab' { constructor() { super(); } }": [
        {
          col: 23,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ],
      "class A extends B { constructor() { } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { for (var a of b) super.foo(); } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var c = class extends D { constructor() { super(); } } } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var c = () => super(); } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { class C extends D { constructor() { super(); } } } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { var C = class extends D { constructor() { super(); } } } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class A extends B { constructor() { super(); super(); } }": [
        {
          col: 45,
          message: too_many_super_message,
          hint: too_many_super_hint,
        }
      ],
      "class A extends B { constructor() { return; super(); } }": [
        {
          col: 20,
          message: no_super_message,
          hint: no_super_hint,
        }
      ],
      "class Foo extends Bar { constructor() { for (a in b) for (c in d); } }": [
        {
          col: 24,
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
          col: 6,
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
          col: 6,
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
          col: 6,
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
          col: 6,
          message: unnecessary_constructor_message,
          hint: unnecessary_constructor_hint,
        }
      ]
    };
  }
}
