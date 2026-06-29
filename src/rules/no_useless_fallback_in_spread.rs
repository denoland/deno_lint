// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::swc::ast::BinaryOp;
use deno_ast::view::{Expr, ObjectLit, PropOrSpread};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoUselessFallbackInSpread;

const CODE: &str = "no-useless-fallback-in-spread";

#[derive(Display)]
enum NoUselessFallbackInSpreadMessage {
  #[display(fmt = "Empty fallbacks in spreads are unnecessary")]
  Unexpected,
}

#[derive(Display)]
enum NoUselessFallbackInSpreadHint {
  #[display(
    fmt = "Spreading falsy values in object literals won't add any unexpected properties, so it's unnecessary to add an empty object as fallback."
  )]
  RemoveFallback,
}

impl LintRule for NoUselessFallbackInSpread {
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
    NoUselessFallbackInSpreadHandler.traverse(program, context);
  }
}

/// Unwraps any number of nested parenthesized expressions.
fn unwrap_parens<'a>(expr: &Expr<'a>) -> Expr<'a> {
  match expr {
    Expr::Paren(paren) => unwrap_parens(&paren.expr),
    _ => *expr,
  }
}

struct NoUselessFallbackInSpreadHandler;

impl Handler for NoUselessFallbackInSpreadHandler {
  fn object_lit(&mut self, object_lit: &ObjectLit, context: &mut Context) {
    for prop in object_lit.props {
      let PropOrSpread::Spread(spread) = prop else {
        continue;
      };

      // The spread argument must be a logical `||` or `??` expression
      // (possibly wrapped in parentheses).
      let Expr::Bin(bin_expr) = unwrap_parens(&spread.expr) else {
        continue;
      };

      if !matches!(
        bin_expr.op(),
        BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
      ) {
        continue;
      }

      // The right operand must be an empty object literal (possibly
      // wrapped in parentheses).
      let Expr::Object(object) = unwrap_parens(&bin_expr.right) else {
        continue;
      };

      if !object.props.is_empty() {
        continue;
      }

      context.add_diagnostic_with_hint(
        spread.range(),
        CODE,
        NoUselessFallbackInSpreadMessage::Unexpected,
        NoUselessFallbackInSpreadHint::RemoveFallback,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_useless_fallback_in_spread.rs
  // MIT Licensed.

  #[test]
  fn no_useless_fallback_in_spread_valid() {
    assert_lint_ok! {
      NoUselessFallbackInSpread,
      "const array = [...(foo || [])]",
      "const array = [...(foo || {})]",
      "const array = [...(foo && {})]",
      "const object = {...(foo && {})}",
      "const object = {...({} || foo)}",
      "const object = {...({} && foo)}",
      "const object = {...({} ?? foo)}",
      "const object = {...(foo ? foo : {})}",
      "const object = {...foo}",
      "const object = {...(foo ?? ({} || {}))}",
      "const {...foo} = object",
      "function foo({...bar}){}",
      "const object = {...(foo || {}).toString()}",
      "const object = {...fn(foo || {})}",
      "const object = call({}, ...(foo || {}))",
      r#"const object = {...(foo || {not: "empty"})}"#,
      "const object = {...(foo || {...{}})}",
    };
  }

  #[test]
  fn no_useless_fallback_in_spread_invalid() {
    assert_lint_err! {
      NoUselessFallbackInSpread,
      "const object = {...(foo || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(foo ?? {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(foo ?? (( {} )))}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...((( foo )) ?? (( {} )))}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(( (( foo )) ?? (( {} )) ))}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "async ()=> ({...((await foo) || {})})": [
        {
          col: 13,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(0 || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...((-0) || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(.0 || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(0n || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(false || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(null || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(undefined || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...((a && b) || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(NaN || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      r#"const object = {...("" || {})}"#: [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...([] || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...({} || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(foo || {}),}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...((foo ?? {}) || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...((foo && {}) || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(foo && {} || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...({...(foo || {})})}": [
        {
          col: 21,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...({...((0, foo) || {})})}": [
        {
          col: 21,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "function foo(a = {...(bar || {})}){}": [
        {
          col: 18,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ],
      "const object = {...(document.all || {})}": [
        {
          col: 16,
          message: NoUselessFallbackInSpreadMessage::Unexpected,
          hint: NoUselessFallbackInSpreadHint::RemoveFallback,
        }
      ]
    };
  }
}
