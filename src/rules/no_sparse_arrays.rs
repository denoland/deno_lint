// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use deno_ast::oxc::ast::ast::{ArrayExpression, ArrayExpressionElement, Program};
use derive_more::Display;

#[derive(Debug)]
pub struct NoSparseArrays;

const CODE: &str = "no-sparse-arrays";

#[derive(Display)]
enum NoSparseArraysMessage {
  #[display(fmt = "Sparse arrays are not allowed")]
  Disallowed,
}

impl LintRule for NoSparseArrays {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoSparseArraysHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoSparseArraysHandler;

impl Handler<'_> for NoSparseArraysHandler {
  fn array_expression(
    &mut self,
    array_lit: &ArrayExpression,
    ctx: &mut Context,
  ) {
    if array_lit
      .elements
      .iter()
      .any(|e| matches!(e, ArrayExpressionElement::Elision(_)))
    {
      ctx.add_diagnostic(
        array_lit.span,
        CODE,
        NoSparseArraysMessage::Disallowed,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_sparse_arrays_valid() {
    assert_lint_ok! {
      NoSparseArrays,
      "const sparseArray1 = [1,null,3];",
    };
  }

  #[test]
  fn no_sparse_arrays_invalid() {
    assert_lint_err! {
      NoSparseArrays,
      r#"const sparseArray = [1,,3];"#: [
      {
        col: 20,
        message: NoSparseArraysMessage::Disallowed,
      }],
    }
  }
}
