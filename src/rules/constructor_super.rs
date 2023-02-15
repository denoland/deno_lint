// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::{view as ast_view, SourceRange, SourceRanged};
use if_chain::if_chain;
use std::sync::Arc;

#[derive(Debug)]
pub struct ConstructorSuper;

const CODE: &str = "constructor-super";

// This rule currently differs from the ESlint implementation
// as there is currently no way of handling code paths in dlint
impl LintRule for ConstructorSuper {
  fn new() -> Arc<Self> {
    Arc::new(ConstructorSuper)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    ConstructorSuperHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/constructor_super.md")
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

fn inherits_from_non_constructor(class: &ast_view::Class) -> bool {
  matches!(&class.super_class, Some(ast_view::Expr::Lit(_)))
}

fn super_call_ranges(constructor: &ast_view::Constructor) -> Vec<SourceRange> {
  if let Some(block_stmt) = &constructor.body {
    block_stmt
      .stmts
      .iter()
      .filter_map(extract_super_range)
      .collect()
  } else {
    vec![]
  }
}

fn extract_super_range(stmt: &ast_view::Stmt) -> Option<SourceRange> {
  if_chain! {
    if let ast_view::Stmt::Expr(expr) = stmt;
    if let ast_view::Expr::Call(call) = expr.expr;
    if matches!(&call.callee, ast_view::Callee::Super(_));
    then {
      Some(call.range())
    } else {
      None
    }
  }
}

fn return_before_super<'a, 'view>(
  constructor: &'a ast_view::Constructor<'view>,
) -> Option<&'a ast_view::ReturnStmt<'view>> {
  if let Some(block_stmt) = &constructor.body {
    for stmt in &block_stmt.stmts {
      if extract_super_range(stmt).is_some() {
        return None;
      }

      if let ast_view::Stmt::Return(ret) = stmt {
        return Some(ret);
      }
    }
  }
  None
}

fn check_constructor(
  cons: &ast_view::Constructor,
  class: &ast_view::Class,
  ctx: &mut Context,
) {
  // Declarations shouldn't be linted
  if cons.body.is_none() {
    return;
  }

  // returning value is a substitute of `super()`.
  if let Some(ret) = return_before_super(cons) {
    if ret.arg.is_none() && class.super_class.is_some() {
      let kind = DiagnosticKind::NoSuper;
      ctx.add_diagnostic_with_hint(
        cons.range(),
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    return;
  }

  if inherits_from_non_constructor(class) {
    let kind = DiagnosticKind::UnnecessaryConstructor;
    ctx.add_diagnostic_with_hint(
      cons.range(),
      CODE,
      kind.message(),
      kind.hint(),
    );
    return;
  }

  let super_calls = super_call_ranges(cons);

  // in case where there are more than one `super()` calls.
  for exceeded_super_range in super_calls.iter().skip(1) {
    let kind = DiagnosticKind::TooManySuper;
    ctx.add_diagnostic_with_hint(
      *exceeded_super_range,
      CODE,
      kind.message(),
      kind.hint(),
    );
  }

  match (super_calls.is_empty(), class.super_class.is_some()) {
    (true, true) => {
      let kind = DiagnosticKind::NoSuper;
      ctx.add_diagnostic_with_hint(
        cons.range(),
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    (false, false) => {
      let kind = DiagnosticKind::UnnecessarySuper;
      ctx.add_diagnostic_with_hint(
        super_calls[0],
        CODE,
        kind.message(),
        kind.hint(),
      );
    }
    _ => {}
  }
}

struct ConstructorSuperHandler;

impl Handler for ConstructorSuperHandler {
  fn class(&mut self, class: &ast_view::Class, ctx: &mut Context) {
    for member in &class.body {
      if let ast_view::ClassMember::Constructor(cons) = member {
        check_constructor(cons, class, ctx);
      }
    }
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

      // https://github.com/denoland/deno_lint/issues/464
      "declare class DOMException extends Error {
        constructor(message?: string, name?: string);
      }"
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
