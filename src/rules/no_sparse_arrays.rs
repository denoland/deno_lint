// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::view::ArrayLit;
use deno_ast::SourceRanged;
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoSparseArrays;

const CODE: &str = "no-sparse-arrays";

#[derive(Display)]
enum NoSparseArraysMessage {
  #[display(fmt = "Sparse arrays are not allowed")]
  Disallowed,
}

impl LintRule for NoSparseArrays {
  fn new() -> Arc<Self> {
    Arc::new(NoSparseArrays)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoSparseArraysHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_sparse_arrays.md")
  }
}

struct NoSparseArraysHandler;

impl Handler for NoSparseArraysHandler {
  fn array_lit(&mut self, array_lit: &ArrayLit, ctx: &mut Context) {
    if array_lit.elems.iter().any(|e| e.is_none()) {
      ctx.add_diagnostic(
        array_lit.range(),
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
