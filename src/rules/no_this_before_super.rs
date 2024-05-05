// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::{view as ast_view, SourceRange, SourceRanged};

#[derive(Debug)]
pub struct NoThisBeforeSuper;

const CODE: &str = "no-this-before-super";
const MESSAGE: &str = "In the constructor of derived classes, `this` / `super` are not allowed before calling to `super()`.";
const HINT: &str = "Call `super()` before using `this` or `super` keyword.";

impl LintRule for NoThisBeforeSuper {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    let mut handler = NoThisBeforeSuperHandler::new();
    handler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_this_before_super.md")
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
  fn on_enter_node(&mut self, node: ast_view::Node, _ctx: &mut Context) {
    if let ast_view::Node::Class(class) = node {
      let is_derived = class.super_class.is_some();
      self.enter_class(is_derived);
    }
  }

  fn on_exit_node(&mut self, node: ast_view::Node, _ctx: &mut Context) {
    if matches!(node, ast_view::Node::Class(_)) {
      self.leave_class();
    }
  }

  fn constructor(&mut self, cons: &ast_view::Constructor, ctx: &mut Context) {
    if !self.inside_derived_class() {
      return;
    }

    if let Some(body) = cons.body {
      for stmt in body.stmts {
        let mut checker = SuperCallChecker::new(stmt.range());
        checker.traverse(*stmt, ctx);
        match checker.result() {
          None => (),
          Some(FirstAppeared::SuperCalled) => break,
          Some(FirstAppeared::ThisAccessed(range))
          | Some(FirstAppeared::SuperAccessed(range)) => {
            ctx.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
          }
        }
      }
    }
  }
}

enum FirstAppeared {
  SuperCalled,
  SuperAccessed(SourceRange),
  ThisAccessed(SourceRange),
}

struct SuperCallChecker {
  first_appeared: Option<FirstAppeared>,
  root_range: SourceRange,
}

impl SuperCallChecker {
  fn new(root_range: SourceRange) -> Self {
    Self {
      first_appeared: None,
      root_range,
    }
  }

  fn yet_appeared(&self) -> bool {
    self.first_appeared.is_none()
  }

  fn result(self) -> Option<FirstAppeared> {
    self.first_appeared
  }

  fn node_is_inside_function(&self, node: ast_view::Node) -> bool {
    fn inside_function(
      root_range: SourceRange,
      cur_node: ast_view::Node,
    ) -> bool {
      // Stop recursion if the current node gets out of root_node.
      if !root_range.contains(&cur_node.range()) {
        return false;
      }

      if matches!(
        cur_node,
        ast_view::Node::Function(_) | ast_view::Node::ArrowExpr(_)
      ) {
        return true;
      }

      inside_function(root_range, cur_node.parent().unwrap())
    }

    inside_function(self.root_range, node)
  }
}

impl Handler for SuperCallChecker {
  fn this_expr(&mut self, this_expr: &ast_view::ThisExpr, _ctx: &mut Context) {
    if self.node_is_inside_function(this_expr.as_node()) {
      return;
    }

    if self.yet_appeared() {
      self.first_appeared =
        Some(FirstAppeared::ThisAccessed(this_expr.range()));
    }
  }

  fn super_(&mut self, super_: &ast_view::Super, _ctx: &mut Context) {
    if self.node_is_inside_function(super_.as_node()) {
      return;
    }

    if self.yet_appeared() {
      self.first_appeared = Some(FirstAppeared::SuperAccessed(super_.range()));
    }
  }

  fn call_expr(&mut self, call_expr: &ast_view::CallExpr, ctx: &mut Context) {
    if self.node_is_inside_function(call_expr.as_node()) {
      return;
    }

    // arguments are evaluated before the callee
    for arg in call_expr.args {
      self.traverse(arg.as_node(), ctx);
    }

    if self.yet_appeared()
      && matches!(call_expr.callee, ast_view::Callee::Super(_))
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
