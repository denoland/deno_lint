// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ExprOrSpread, Lit, MemberProp};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NumberArgOutOfRange;

const CODE: &str = "number-arg-out-of-range";

impl LintRule for NumberArgOutOfRange {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NumberArgOutOfRangeHandler.traverse(program, context);
  }
}

struct NumberArgOutOfRangeHandler;

impl Handler for NumberArgOutOfRangeHandler {
  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    let Callee::Expr(callee) = call_expr.callee else {
      return;
    };
    let Expr::Member(member) = callee else {
      return;
    };
    let Some(name) = static_member_name(&member.prop) else {
      return;
    };

    let (min, max) = match name.as_str() {
      "toString" => (2, 36),
      "toFixed" | "toExponential" => (0, 20),
      "toPrecision" => (1, 21),
      _ => return,
    };

    let Some(ExprOrSpread { expr, .. }) = call_expr.args.first() else {
      return;
    };
    let Expr::Lit(Lit::Num(num)) = expr else {
      return;
    };

    let value = num.inner.value;
    if value < f64::from(min) || value > f64::from(max) {
      ctx.add_diagnostic_with_hint(
        call_expr.range(),
        CODE,
        format!("The argument to `{name}` must be between {min} and {max}"),
        format!("Pass a value between {min} and {max}"),
      );
    }
  }
}

/// Returns the statically known property name of a member expression, handling
/// both `obj.name` and `obj["name"]` forms.
fn static_member_name(prop: &MemberProp) -> Option<String> {
  match prop {
    MemberProp::Ident(ident) => Some(ident.sym().to_string()),
    MemberProp::Computed(computed) => match computed.expr {
      Expr::Lit(Lit::Str(s)) => Some(s.value().to_string_lossy().into_owned()),
      _ => None,
    },
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/oxc/number_arg_out_of_range.rs
  // MIT Licensed.

  #[test]
  fn number_arg_out_of_range_valid() {
    assert_lint_ok! {
      NumberArgOutOfRange,
      "x.toString(16);",
      "x.toFixed(2);",
      "x.toExponential(0);",
      "x.toPrecision(10);",
      // No argument.
      "x.toString();",
      // Non-literal argument.
      "x.toString(y);",
      // Unrelated method.
      "x.slice(64);",
    };
  }

  #[test]
  fn number_arg_out_of_range_invalid() {
    assert_lint_err! {
      NumberArgOutOfRange,
      "x.toString(1);": [
        {
          col: 0,
          message: "The argument to `toString` must be between 2 and 36",
          hint: "Pass a value between 2 and 36",
        }
      ],
      "x.toString(43);": [
        {
          col: 0,
          message: "The argument to `toString` must be between 2 and 36",
          hint: "Pass a value between 2 and 36",
        }
      ],
      "x.toFixed(22);": [
        {
          col: 0,
          message: "The argument to `toFixed` must be between 0 and 20",
          hint: "Pass a value between 0 and 20",
        }
      ],
      "x.toPrecision(0);": [
        {
          col: 0,
          message: "The argument to `toPrecision` must be between 1 and 21",
          hint: "Pass a value between 1 and 21",
        }
      ],
      "x.toPrecision(100);": [
        {
          col: 0,
          message: "The argument to `toPrecision` must be between 1 and 21",
          hint: "Pass a value between 1 and 21",
        }
      ],
      // Computed member access.
      "x['toExponential'](22);": [
        {
          col: 0,
          message: "The argument to `toExponential` must be between 0 and 20",
          hint: "Pass a value between 0 and 20",
        }
      ]
    };
  }
}
