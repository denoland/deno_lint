// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_util::find_lhs_ids;
use crate::ProgramRef;
use deno_ast::swc::ast::AssignExpr;
use deno_ast::SwcSourceRanged;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::{VisitAll, VisitAllWith};
use deno_ast::BindingKind;
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoFuncAssign;

const CODE: &str = "no-func-assign";

#[derive(Display)]
enum NoFuncAssignMessage {
  #[display(fmt = "Reassigning function declaration is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoFuncAssignHint {
  #[display(
    fmt = "Remove or rework the reassignment of the existing function"
  )]
  RemoveOrRework,
}

impl LintRule for NoFuncAssign {
  fn new() -> Arc<Self> {
    Arc::new(NoFuncAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoFuncAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_func_assign.md")
  }
}

struct NoFuncAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoFuncAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoFuncAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr) {
    let ids = find_lhs_ids(&assign_expr.left);

    for id in ids {
      let var = self.context.scope().var(&id);
      if let Some(var) = var {
        if let BindingKind::Function = var.kind() {
          self.context.add_diagnostic_with_hint(
            assign_expr.range(),
            CODE,
            NoFuncAssignMessage::Unexpected,
            NoFuncAssignHint::RemoveOrRework,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.13.0/tests/lib/rules/no-func-assign.js
  // MIT Licensed.

  #[test]
  fn no_func_assign_valid() {
    assert_lint_ok! {
      NoFuncAssign,
      "function foo() { var foo = bar; }",
      "function foo(foo) { foo = bar; }",
      "function foo() { var foo; foo = bar; }",
      "var foo = () => {}; foo = bar;",
      "var foo = function() {}; foo = bar;",
      "var foo = function() { foo = bar; };",
      "import bar from 'bar'; function foo() { var foo = bar; }",
    };
  }

  #[test]
  fn no_func_assign_invalid() {
    assert_lint_err! {
      NoFuncAssign,
      "function foo() {}; foo = bar;": [
        {
          col: 19,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { foo = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "foo = bar; function foo() { };": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "[foo] = bar; function foo() { }": [
        {
          col: 0,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "({x: foo = 0} = bar); function foo() { };": [
        {
          col: 1,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "function foo() { [foo] = bar; }": [
        {
          col: 17,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "(function() { ({x: foo = 0} = bar); function foo() { }; })();": [
        {
          col: 15,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      "var a = function foo() { foo = 123; };": [
        {
          col: 25,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
      r#"
const a = "a";
const unused = "unused";

function asdf(b: number, c: string): number {
    console.log(a, b);
    debugger;
    return 1;
}

asdf = "foobar";
      "#: [
        {
          col: 0,
          line: 11,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],

      // nested
      r#"
function foo() {}
let a;
a = () => {
  foo = 42;
};
      "#: [
        {
          line: 5,
          col: 2,
          message: NoFuncAssignMessage::Unexpected,
          hint: NoFuncAssignHint::RemoveOrRework,
        }
      ],
    };
  }
}
