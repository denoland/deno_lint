// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  AssignmentExpression, AssignmentTarget, AssignmentTargetMaybeDefault,
  AssignmentTargetProperty, IdentifierReference, Program,
};
use deno_ast::BindingKind;

#[derive(Debug)]
pub struct NoClassAssign;

const CODE: &str = "no-class-assign";
const MESSAGE: &str = "Reassigning class declaration is not allowed";
const HINT: &str = "Do you have the right variable here?";

impl LintRule for NoClassAssign {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoClassAssignVisitor;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoClassAssignVisitor;

fn collect_target_ident_refs<'a>(
  target: &'a AssignmentTarget<'a>,
  refs: &mut Vec<&'a IdentifierReference<'a>>,
) {
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => {
      refs.push(ident);
    }
    AssignmentTarget::ArrayAssignmentTarget(array) => {
      for elem in array.elements.iter().flatten() {
        collect_maybe_default_ident_refs(elem, refs);
      }
      if let Some(rest) = &array.rest {
        collect_target_ident_refs(&rest.target, refs);
      }
    }
    AssignmentTarget::ObjectAssignmentTarget(object) => {
      for prop in &object.properties {
        match prop {
          AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(p) => {
            refs.push(&p.binding);
          }
          AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
            collect_maybe_default_ident_refs(&p.binding, refs);
          }
        }
      }
      if let Some(rest) = &object.rest {
        collect_target_ident_refs(&rest.target, refs);
      }
    }
    _ => {}
  }
}

fn collect_maybe_default_ident_refs<'a>(
  target: &'a AssignmentTargetMaybeDefault<'a>,
  refs: &mut Vec<&'a IdentifierReference<'a>>,
) {
  match target {
    AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(d) => {
      collect_target_ident_refs(&d.binding, refs);
    }
    AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(ident) => {
      refs.push(ident);
    }
    _ => {}
  }
}

impl Handler<'_> for NoClassAssignVisitor {
  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    let mut refs = Vec::new();
    collect_target_ident_refs(&assign_expr.left, &mut refs);
    let should_report = refs.iter().any(|ident_ref| {
      matches!(
        ctx.binding_kind_of_ident_ref(ident_ref),
        Some(BindingKind::Class)
      )
    });
    if should_report {
      ctx.add_diagnostic_with_hint(assign_expr.span, CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/no-class-assign.js
  // MIT Licensed.

  #[test]
  fn no_class_assign_valid() {
    assert_lint_ok! {
      NoClassAssign,
      r#"class A {}"#,
      r#"class A {} foo(A);"#,
      r#"let A = class A {}; foo(A);"#,
      r#"
class A {
  foo(A) {
    A = "foobar";
  }
}
"#,
      r#"
class A {
  foo() {
    let A;
    A = "bar";
  }
}
"#,
      r#"
let A = class {
  b() {
    A = 0;
  }
}
"#,
      r#"
let A, B;
A = class {
  b() {
    B = 0;
  }
}
"#,
      r#"let x = 0; x = 1;"#,
      r#"var x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"function x() {} x = 1;"#,
      r#"function foo(x) { x = 1; }"#,
      r#"try {} catch (x) { x = 1; }"#,
    };
  }

  #[test]
  fn no_class_assign_invalid() {
    assert_lint_err! {
      NoClassAssign,
      r#"
class A {}
A = 0;
      "#: [
        {
          line: 3,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {}
({A} = 0);
      "#: [
        {
          line: 3,
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {}
({b: A = 0} = {});
      "#: [
        {
          line: 3,
          col: 1,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
A = 0;
class A {}
      "#: [
        {
          line: 2,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {
  foo() {
    A = 0;
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
let A = class A {
  foo() {
    A = 0;
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
class A {}
A = 10;
A = 20;
      "#: [
        {
          line: 3,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        },
        {
          line: 4,
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
let A;
A = class {
  foo() {
    class B {}
    B = 0;
  }
}
      "#: [
        {
          line: 6,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
