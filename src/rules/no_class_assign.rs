// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::find_lhs_ids;
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::AssignExpr;
use deno_ast::{BindingKind, SourceRanged};

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

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    NoClassAssignVisitor.traverse(program, context);
  }
}

struct NoClassAssignVisitor;

impl Handler for NoClassAssignVisitor {
  fn assign_expr(&mut self, assign_expr: &AssignExpr, ctx: &mut Context) {
    let ids = find_lhs_ids(&assign_expr.left);
    for id in ids {
      let var = ctx.scope().var(&id);
      if let Some(var) = var {
        if let BindingKind::Class = var.kind() {
          ctx.add_diagnostic_with_hint(
            assign_expr.range(),
            CODE,
            MESSAGE,
            HINT,
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
