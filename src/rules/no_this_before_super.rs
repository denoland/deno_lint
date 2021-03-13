// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait, Span, Spanned};

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

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    let mut handler = NoThisBeforeSuperHandler::new();
    handler.traverse(program, context);
  }
}

struct NoThisBeforeSuperHandler {
  /// Stores bools that represent whether classes are derived one or not.
  /// When it enters a class, a bool value is pushed into this vector. And when it leaves the
  /// class, pop the last value of vector.
  /// The last value of the vector indicates whether we are now in a derived class or not.
  is_derived_class: Vec<bool>,
}

impl NoThisBeforeSuperHandler {
  fn new() -> Self {
    Self {
      is_derived_class: Vec::new(),
    }
  }

  fn enter_class(&mut self, is_derived: bool) {
    self.is_derived_class.push(is_derived);
  }

  fn leave_class(&mut self) {
    assert!(!self.is_derived_class.is_empty());
    self.is_derived_class.pop();
  }

  fn inside_derived_class(&self) -> bool {
    match self.is_derived_class.as_slice() {
      [] => false,
      [x] => *x,
      [.., x] => *x,
    }
  }
}

impl Handler for NoThisBeforeSuperHandler {
  fn on_enter_node(&mut self, node: AstView::Node, _ctx: &mut Context) {
    if let AstView::Node::Class(ref class) = node {
      let is_derived = class.super_class.is_some();
      self.enter_class(is_derived);
    }
  }

  fn on_exit_node(&mut self, node: AstView::Node, _ctx: &mut Context) {
    if matches!(node, AstView::Node::Class(_)) {
      self.leave_class();
    }
  }

  fn constructor(&mut self, cons: &AstView::Constructor, ctx: &mut Context) {
    if !self.inside_derived_class() {
      return;
    }

    if let Some(body) = cons.body {
      for stmt in &body.stmts {
        let mut checker = SuperCallChecker::new(stmt.span());
        checker.traverse(*stmt, ctx);
        match checker.result() {
          None => (),
          Some(FirstAppeared::SuperCalled) => break,
          Some(FirstAppeared::ThisAccessed(span))
          | Some(FirstAppeared::SuperAccessed(span)) => {
            ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
          }
        }
      }
    }
  }
}

enum FirstAppeared {
  SuperCalled,
  SuperAccessed(Span),
  ThisAccessed(Span),
}

struct SuperCallChecker {
  first_appeared: Option<FirstAppeared>,
  root_span: Span,
}

impl SuperCallChecker {
  fn new(root_span: Span) -> Self {
    Self {
      first_appeared: None,
      root_span,
    }
  }

  fn yet_appeared(&self) -> bool {
    self.first_appeared.is_none()
  }

  fn result(self) -> Option<FirstAppeared> {
    self.first_appeared
  }

  fn node_is_inside_function(&self, node: AstView::Node) -> bool {
    fn inside_function(root_span: Span, cur_node: AstView::Node) -> bool {
      // Stop recursion if the current node gets out of root_node.
      if !root_span.contains(cur_node.span()) {
        return false;
      }

      if matches!(
        cur_node,
        AstView::Node::Function(_) | AstView::Node::ArrowExpr(_)
      ) {
        return true;
      }

      inside_function(root_span, cur_node.parent().unwrap())
    }

    inside_function(self.root_span, node)
  }
}

impl Handler for SuperCallChecker {
  fn this_expr(&mut self, this_expr: &AstView::ThisExpr, _ctx: &mut Context) {
    if self.node_is_inside_function(this_expr.into_node()) {
      return;
    }

    if self.yet_appeared() {
      self.first_appeared = Some(FirstAppeared::ThisAccessed(this_expr.span()));
    }
  }

  fn super_(&mut self, super_: &AstView::Super, _ctx: &mut Context) {
    if self.node_is_inside_function(super_.into_node()) {
      return;
    }

    if self.yet_appeared() {
      self.first_appeared = Some(FirstAppeared::SuperAccessed(super_.span()));
    }
  }

  fn call_expr(&mut self, call_expr: &AstView::CallExpr, ctx: &mut Context) {
    if self.node_is_inside_function(call_expr.into_node()) {
      return;
    }

    // arguments are evaluated before the callee
    for arg in &call_expr.args {
      self.traverse(arg.into_node(), ctx);
    }

    if self.yet_appeared()
      && matches!(call_expr.callee, AstView::ExprOrSuper::Super(_))
    {
      self.first_appeared = Some(FirstAppeared::SuperCalled);
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
      r#"
class A extends B {
  constructor() {
    function foo() {
      this.bar();
    }
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    const foo = () => {
      this.bar();
    };
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    super({
      foo() {
        this.bar();
      }
    });
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
