// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  BinExpr, BinaryOp, Callee, Expr, Lit, MemberProp, Node, NodeTrait,
};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoUselessLengthCheck;

const CODE: &str = "no-useless-length-check";

#[derive(Display)]
enum NoUselessLengthCheckMessage {
  #[display(fmt = "Found a useless array length check")]
  Useless,
}

#[derive(Display)]
enum NoUselessLengthCheckHint {
  #[display(
    fmt = "The empty check is useless as `Array#every()` returns `true` for an empty array."
  )]
  Every,
  #[display(
    fmt = "The non-empty check is useless as `Array#some()` returns `false` for an empty array."
  )]
  Some,
}

impl LintRule for NoUselessLengthCheck {
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
    NoUselessLengthCheckHandler.traverse(program, context);
  }
}

struct NoUselessLengthCheckHandler;

impl Handler for NoUselessLengthCheckHandler {
  fn bin_expr(&mut self, bin_expr: &BinExpr, context: &mut Context) {
    let op = bin_expr.op();
    if op != BinaryOp::LogicalOr && op != BinaryOp::LogicalAnd {
      return;
    }

    // Only process the root of a same-operator chain. `A || B || C` parses as
    // `((A || B) || C)`, and every logical `BinExpr` triggers this handler,
    // including the inner `A || B`. Since the outer node flattens the whole
    // chain and checks every adjacent pair, processing the inner nodes too
    // would report shared pairs multiple times. If this node is itself an
    // operand of a parent logical expression with the same operator (ignoring
    // surrounding parentheses), bail out and let the root handle it.
    let mut parent = bin_expr.parent();
    while let Node::ParenExpr(paren) = parent {
      parent = paren.parent();
    }
    if let Node::BinExpr(parent_bin) = parent {
      if parent_bin.op() == op {
        return;
      }
    }

    let mut flat = Vec::new();
    flatten(bin_expr, op, &mut flat);

    for window in flat.windows(2) {
      if let Some((range, is_every)) =
        is_useless_check(window[0], window[1], op)
      {
        if is_every {
          context.add_diagnostic_with_hint(
            range,
            CODE,
            NoUselessLengthCheckMessage::Useless,
            NoUselessLengthCheckHint::Every,
          );
        } else {
          context.add_diagnostic_with_hint(
            range,
            CODE,
            NoUselessLengthCheckMessage::Useless,
            NoUselessLengthCheckHint::Some,
          );
        }
      }
    }
  }
}

fn unwrap_paren(mut expr: Expr) -> Expr {
  while let Expr::Paren(paren) = expr {
    expr = paren.expr;
  }
  expr
}

/// Flattens a chain of logical expressions that share the same operator into a
/// flat list of operands, mirroring oxc's `make_flat_logical_expression`.
fn flatten<'a>(node: &BinExpr<'a>, op: BinaryOp, out: &mut Vec<Expr<'a>>) {
  push_side(node.left, op, out);
  push_side(node.right, op, out);
}

fn push_side<'a>(expr: Expr<'a>, op: BinaryOp, out: &mut Vec<Expr<'a>>) {
  match unwrap_paren(expr) {
    Expr::Bin(bin) if bin.op() == op => flatten(bin, op, out),
    other => out.push(other),
  }
}

/// If `expr` is a length check like `array.length === 0` (with one of the
/// allowed operators and a raw `0` literal), returns the array's identifier
/// name and the range of the check.
fn as_length_check<'a>(
  expr: Expr<'a>,
  allowed_ops: &[BinaryOp],
) -> Option<(String, SourceRange)> {
  let Expr::Bin(bin) = expr else {
    return None;
  };
  if !allowed_ops.contains(&bin.op()) {
    return None;
  }
  if !is_raw_zero(bin.right) {
    return None;
  }
  let Expr::Member(member) = unwrap_paren(bin.left) else {
    return None;
  };
  let Expr::Ident(obj) = member.obj else {
    return None;
  };
  let MemberProp::Ident(prop) = member.prop else {
    return None;
  };
  if prop.sym().as_str() != "length" {
    return None;
  }
  Some((obj.sym().to_string(), bin.range()))
}

/// If `expr` is a call like `array.every(...)` / `array.some(...)` with the
/// given method name, returns the array's identifier name.
fn as_array_method_call<'a>(expr: Expr<'a>, method: &str) -> Option<String> {
  let Expr::Call(call) = expr else {
    return None;
  };
  let Callee::Expr(callee) = call.callee else {
    return None;
  };
  let Expr::Member(member) = unwrap_paren(callee) else {
    return None;
  };
  let Expr::Ident(obj) = member.obj else {
    return None;
  };
  let MemberProp::Ident(prop) = member.prop else {
    return None;
  };
  if prop.sym().as_str() != method {
    return None;
  }
  Some(obj.sym().to_string())
}

fn is_raw_zero(expr: Expr) -> bool {
  matches!(expr, Expr::Lit(Lit::Num(num)) if num.text() == "0")
}

/// Checks an adjacent pair of operands. Returns the range to report and whether
/// the `every` (vs `some`) message applies.
fn is_useless_check(
  left: Expr,
  right: Expr,
  op: BinaryOp,
) -> Option<(SourceRange, bool)> {
  let is_every = op == BinaryOp::LogicalOr;
  let (method, allowed_ops): (&str, &[BinaryOp]) = if is_every {
    ("every", &[BinaryOp::EqEqEq])
  } else {
    ("some", &[BinaryOp::NotEqEq, BinaryOp::Gt])
  };

  if let (Some((left_name, range)), Some(right_name)) = (
    as_length_check(left, allowed_ops),
    as_array_method_call(right, method),
  ) {
    if left_name == right_name {
      return Some((range, is_every));
    }
  }

  if let (Some(left_name), Some((right_name, range))) = (
    as_array_method_call(left, method),
    as_length_check(right, allowed_ops),
  ) {
    if left_name == right_name {
      return Some((range, is_every));
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_useless_length_check.rs
  // MIT Licensed.

  #[test]
  fn no_useless_length_check_valid() {
    assert_lint_ok! {
      NoUselessLengthCheck,
      "array.length === 0 ?? array.every(Boolean)",
      "array.length === 0 && array.every(Boolean)",
      "(array.length === 0) + (array.every(Boolean))",
      "array.length === 1 || array.every(Boolean)",
      r#"array.length === "0" || array.every(Boolean)"#,
      "array.length === 0. || array.every(Boolean)",
      "array.length === 0x0 || array.every(Boolean)",
      "array.length !== 0 || array.every(Boolean)",
      "array.length == 0 || array.every(Boolean)",
      "0 === array.length || array.every(Boolean)",
      "array?.length === 0 || array.every(Boolean)",
      "array.notLength === 0 || array.every(Boolean)",
      "array[length] === 0 || array.every(Boolean)",
      "array.length === 0 || array.every?.(Boolean)",
      "array.length === 0 || array?.every(Boolean)",
      "array.length === 0 || array.every",
      "array.length === 0 || array[every](Boolean)",
      "array1.length === 0 || array2.every(Boolean)",
      "array.length !== 0 ?? array.some(Boolean)",
      "array.length !== 0 || array.some(Boolean)",
      "(array.length !== 0) - (array.some(Boolean))",
      "array.length !== 1 && array.some(Boolean)",
      r#"array.length !== "0" && array.some(Boolean)"#,
      "array.length !== 0. && array.some(Boolean)",
      "array.length !== 0x0 && array.some(Boolean)",
      "array.length === 0 && array.some(Boolean)",
      "array.length <= 0 && array.some(Boolean)",
      "array.length != 0 && array.some(Boolean)",
      "0 !== array.length && array.some(Boolean)",
      "array?.length !== 0 && array.some(Boolean)",
      "array.notLength !== 0 && array.some(Boolean)",
      "array[length] !== 0 && array.some(Boolean)",
      "array.length !== 0 && array.some?.(Boolean)",
      "array.length !== 0 && array?.some(Boolean)",
      "array.length !== 0 && array.some",
      "array.length !== 0 && array.notSome(Boolean)",
      "array.length !== 0 && array[some](Boolean)",
      "array1.length !== 0 && array2.some(Boolean)",
      "array.length > 0 ?? array.some(Boolean)",
      "array.length > 0 || array.some(Boolean)",
      "(array.length > 0) - (array.some(Boolean))",
      "array.length > 1 && array.some(Boolean)",
      r#"array.length > "0" && array.some(Boolean)"#,
      "array.length > 0. && array.some(Boolean)",
      "array.length > 0x0 && array.some(Boolean)",
      "array.length >= 0 && array.some(Boolean)",
      "0 > array.length && array.some(Boolean)",
      "0 < array.length && array.some(Boolean)",
      "array?.length > 0 && array.some(Boolean)",
      "array.notLength > 0 && array.some(Boolean)",
      "array.length > 0 && array.some?.(Boolean)",
      "array.length > 0 && array?.some(Boolean)",
      "array.length > 0 && array.some",
      "array.length > 0 && array.notSome(Boolean)",
      "array.length > 0 && array[some](Boolean)",
      "array1.length > 0 && array2.some(Boolean)",
      "if (
                foo &&
                array.length !== 0 &&
                bar &&
                array.some(Boolean)
            ) {
                // ...
            }",
      "(foo && array.length === 0) || array.every(Boolean) && foo",
      "array.length === 0 || (array.every(Boolean) && foo)",
      "(foo || array.length > 0) && array.some(Boolean)",
      "array.length > 0 && (array.some(Boolean) || foo)",
    };
  }

  #[test]
  fn no_useless_length_check_invalid() {
    assert_lint_err! {
      NoUselessLengthCheck,
      "array.length === 0 || array.every(Boolean)": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "array.length > 0 && array.some(Boolean)": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "array.length !== 0 && array.some(Boolean)": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "if ((( array.length > 0 )) && array.some(Boolean));": [
        {
          col: 7,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "if (
                array.length !== 0 &&
                array.some(Boolean) &&
                foo
            ) {
                // ...
            }": [
        {
          line: 2,
          col: 16,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "(array.length === 0 || array.every(Boolean)) || foo": [
        {
          col: 1,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "foo || (array.length === 0 || array.every(Boolean))": [
        {
          col: 8,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "(array.length > 0 && array.some(Boolean)) && foo": [
        {
          col: 1,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "foo && (array.length > 0 && array.some(Boolean))": [
        {
          col: 8,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "array.every(Boolean) || array.length === 0": [
        {
          col: 24,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "array.some(Boolean) && array.length !== 0": [
        {
          col: 23,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "array.some(Boolean) && array.length > 0": [
        {
          col: 23,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "foo && array.length > 0 && array.some(Boolean)": [
        {
          col: 7,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "foo || array.length === 0 || array.every(Boolean)": [
        {
          col: 7,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "(foo || array.length === 0) || array.every(Boolean)": [
        {
          col: 8,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "array.length === 0 || (array.every(Boolean) || foo)": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "(foo && array.length > 0) && array.some(Boolean)": [
        {
          col: 8,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "array.length > 0 && (array.some(Boolean) && foo)": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ],
      "array.every(Boolean) || array.length === 0 || array.every(Boolean)": [
        {
          col: 24,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        },
        {
          col: 24,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "array.length === 0 || array.every(Boolean) || array.length === 0": [
        {
          col: 0,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        },
        {
          col: 46,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "array1.every(Boolean)
            || (( array1.length === 0 || array2.length === 0 )) // Both useless
            || array2.every(Boolean)": [
        {
          line: 2,
          col: 18,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        },
        {
          line: 2,
          col: 41,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Every,
        }
      ],
      "function isUselessLengthCheckNode({node, operator, siblings}) {
                return (
                    (
                        operator === '||' &&
                        zeroLengthChecks.has(node) &&
                        siblings.length > 0 &&
                        siblings.some(condition =>
                            arrayEveryCalls.has(condition) &&
                            isSameReference(node.left.object, condition.callee.object)
                        )
                    ) ||
                    (
                        operator === '&&' &&
                        nonZeroLengthChecks.has(node) &&
                        siblings.length > 0 &&
                        siblings.some(condition =>
                            arraySomeCalls.has(condition) &&
                            isSameReference(node.left.object, condition.callee.object)
                        )
                    )
                );
            }": [
        {
          line: 6,
          col: 24,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        },
        {
          line: 15,
          col: 24,
          message: NoUselessLengthCheckMessage::Useless,
          hint: NoUselessLengthCheckHint::Some,
        }
      ]
    };
  }
}
