// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Expr, NewExpr};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoNewArray;

const CODE: &str = "no-new-array";

#[derive(Display)]
enum NoNewArrayMessage {
  #[display(fmt = "Do not use `new Array(singleArgument)`.")]
  Unexpected,
}

#[derive(Display)]
enum NoNewArrayHint {
  #[display(
    fmt = "It's not clear whether the argument is meant to be the length of the array or the only element. If the argument is the array's length, consider using `Array.from({{ length: n }})`. If the argument is the only element, use `[element]`."
  )]
  ArrayFromOrLiteral,
}

impl LintRule for NoNewArray {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoNewArrayHandler.traverse(program, context);
  }
}

struct NoNewArrayHandler;

impl Handler for NoNewArrayHandler {
  fn new_expr(&mut self, new_expr: &NewExpr, context: &mut Context) {
    let Expr::Ident(ident) = &new_expr.callee else {
      return;
    };

    if ident.inner.as_ref() != "Array" {
      return;
    }

    let Some(args) = &new_expr.args else {
      return;
    };

    if args.len() != 1 {
      return;
    }

    context.add_diagnostic_with_hint(
      new_expr.range(),
      CODE,
      NoNewArrayMessage::Unexpected,
      NoNewArrayHint::ArrayFromOrLiteral,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_new_array.rs
  // MIT Licensed.

  #[test]
  fn no_new_array_valid() {
    assert_lint_ok! {
      NoNewArray,
      "const array = Array.from({length: 1})",
      "const array = new Array()",
      "const array = new Array",
      "const array = new Array(1, 2)",
      "const array = Array(1, 2)",
      "const array = Array(1)",
    };
  }

  #[test]
  fn no_new_array_invalid() {
    assert_lint_err! {
      NoNewArray,
      "const array = new Array(1)": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const zero = 0;\n            const array = new Array(zero);": [
        { line: 2, col: 26, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const length = 1;\n            const array = new Array(length);": [
        { line: 2, col: 26, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const array = new Array(1.5)": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const array = new Array(Number("1"))"#: [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const array = new Array("1")"#: [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const array = new Array(null)": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const array = new Array(("1"))"#: [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const array = new Array((0, 1))": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const foo = []\n            new Array(\"bar\").forEach(baz)": [
        { line: 2, col: 12, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(0xff)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Math.PI | foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Math.min(foo, bar))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Number(foo))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Number.MAX_SAFE_INTEGER)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(parseInt(foo))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Number.parseInt(foo))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(+foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(-Math.PI)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"new Array(-"-2")"#: [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo.length)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const foo = 1; new Array(foo + 2)": [
        { col: 15, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo - 2)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo -= 2)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo ? 1 : 2)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const truthy = "truthy"; new Array(truthy ? 1 : foo)"#: [
        { col: 25, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const falsy = !"truthy"; new Array(falsy ? foo : 1)"#: [
        { col: 25, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array((1n, 2))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Number.NaN)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(NaN)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo >>> bar)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo >>>= bar)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(++bar.length)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(bar.length++)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo = bar.length)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"new Array("0xff")"#: [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Math.NON_EXISTS_PROPERTY)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Math.NON_EXISTS_METHOD(foo))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Math[min](foo, bar))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(Number[MAX_SAFE_INTEGER])": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(new Number(foo))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const foo = 1; new Array(foo + "2")"#: [
        { col: 15, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo - 2n)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo -= 2n)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo instanceof 1)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo || 1)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo ||= 1)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(foo ? 1n : 2)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array((1, 2n))": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(-foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(~foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(typeof 1)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const truthy = "truthy"; new Array(truthy ? foo : 1)"#: [
        { col: 25, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      r#"const falsy = !"truthy"; new Array(falsy ? 1 : foo)"#: [
        { col: 25, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(unknown ? foo : 1)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(unknown ? 1 : foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "new Array(++foo)": [
        { col: 0, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const array = new Array(foo)": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const array = new Array(length)": [
        { col: 14, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const foo = []\n            new Array(bar).forEach(baz)": [
        { line: 2, col: 12, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
      "const foo = []\n            new Array(...bar).forEach(baz)": [
        { line: 2, col: 12, message: NoNewArrayMessage::Unexpected, hint: NoNewArrayHint::ArrayFromOrLiteral }
      ],
    };
  }
}
