// Copyright 2020-2022 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Spanned;
use deno_ast::view as ast_view;
use if_chain::if_chain;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoUnneededTernary;

const CODE: &str = "no-unneeded-ternary";
const MESSAGE: &str =
  "Unnecessary use of boolean literals in conditional expression";

impl LintRule for NoUnneededTernary {
  fn new() -> Arc<Self> {
    Arc::new(NoUnneededTernary)
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
    program: Program<'_>,
  ) {
    NoUnneededTernaryHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unneeded_ternary.md")
  }
}

struct NoUnneededTernaryHandler;

impl Handler for NoUnneededTernaryHandler {
  fn cond_expr(&mut self, cond_expr: &ast_view::CondExpr, ctx: &mut Context) {
    if_chain! {
      if cond_expr.cons.is::<ast_view::Bool>();
      if cond_expr.alt.is::<ast_view::Bool>();
      then {
        ctx.add_diagnostic(cond_expr.span(), CODE, MESSAGE);
      }
    }
  }
}

// https://github.com/eslint/eslint/blob/main/tests/lib/rules/no-unneeded-ternary.js
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unneeded_ternary_valid() {
    assert_lint_ok! {
      NoUnneededTernary,
      r#"config.newIsCap = config.newIsCap !== false"#,
      r#"var a = x === 2 ? 'Yes' : 'No';"#,
      r#"var a = x === 2 ? true : 'No';"#,
      r#"var a = x === 2 ? 'Yes' : false;"#,
      r#"var a = x === 2 ? 'true' : 'false';"#,
    };
  }

  #[test]
  fn no_unneeded_ternary_invalid() {
    assert_lint_err! {
      NoUnneededTernary,
      r#"x === 2 ? true : false;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"x >= 2 ? true : false;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"x ? true : false;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"x === 1 ? false : true;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"x != 1 ? false : true;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"foo() ? false : true;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"!foo() ? false : true;"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
    };
  }
}
