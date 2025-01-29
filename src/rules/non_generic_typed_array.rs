// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::tags::Tags;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NonGenericTypedArray;

const CODE: &str = "non-generic-typed-array";
const MESSAGE: &str = "Forces making typed arrays like Uint8Array generic when in non-input types.";
const HINT: &str = "Add a type parameter (ex. Uint8Array<ArrayBuffer>)";
const FIX_DESC_ARRAY_BUFFER: &str = "Add <ArrayBuffer>";
const FIX_DESC_SHARED_ARRAY_BUFFER: &str = "Add <SharedArrayBuffer>";
const FIX_DESC_ARRAY_BUFFER_LIKE: &str = "Add <ArrayBufferLike>";

const TYPED_ARRAYS: [&str; 11] = [
  "BigInt64Array",
  "BigUint64Array",
  "Float32Array",
  "Float64Array",
  "Int16Array",
  "Int32Array",
  "Int8Array",
  "Uint16Array",
  "Uint32Array",
  "Uint8Array",
  "Uint8ClampedArray",
];

impl LintRule for NonGenericTypedArray {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NonGenericTypedArrayHandler.traverse(program, context);
  }
}

struct NonGenericTypedArrayHandler;

impl NonGenericTypedArrayHandler {
  fn add_diagnostic(&self, ctx: &mut Context, name: &str, range: SourceRange) {
    ctx.add_diagnostic_with_fixes(
      range,
      CODE,
      MESSAGE,
      Some(HINT.to_string()),
      vec![LintFix {
        description: FIX_DESC_ARRAY_BUFFER.into(),
        changes: vec![LintFixChange {
          new_text: format!("{}<ArrayBuffer>", name).into(),
          range,
        }],
      }, LintFix {
        description: FIX_DESC_ARRAY_BUFFER_LIKE.into(),
        changes: vec![LintFixChange {
          new_text: format!("{}<ArrayBufferLike>", name).into(),
          range,
        }],
      }, LintFix {
        description: FIX_DESC_SHARED_ARRAY_BUFFER.into(),
        changes: vec![LintFixChange {
          new_text: format!("{}<SharedArrayBuffer>", name).into(),
          range,
        }],
      }],
    );
  }
}

impl Handler for NonGenericTypedArrayHandler {
  fn param(&mut self, _n: &ast_view::Param, ctx: &mut Context) {
    // don't analyze in params
     ctx.stop_traverse();
  }

  fn ts_type_ref(&mut self, n: &ast_view::TsTypeRef, ctx: &mut Context) {
    if n.type_params.is_none() {
      if let ast_view::TsEntityName::Ident(ident) = &n.type_name {
        if TYPED_ARRAYS.binary_search(&ident.sym().as_str()).is_ok() {
          self.add_diagnostic(ctx, ident.sym().as_str(), n.type_name.range());
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ensure_typed_arrays_sorted() {
    let mut sorted = TYPED_ARRAYS.iter().collect::<Vec<_>>();
    sorted.sort();
    assert_eq!(sorted, TYPED_ARRAYS.iter().collect::<Vec<_>>());
  }

  #[test]
  fn no_window_valid() {
    assert_lint_ok! {
      NonGenericTypedArray,
      "type Test = Uint8Array<ArrayBuffer>;",
      "type Test = Uint8Array<ArrayBufferLike>;",
      "function test(): Uint8Array<ArrayBufferLike> {}",

      // ok to accept a wide input type
      "function value(typed: Uint8Array) {}",
      "class Class { method(typed: Uint8Array) {} }",
    };
  }

  #[test]
  fn no_window_invalid() {
    assert_lint_err! {
      NonGenericTypedArray,
      MESSAGE,
      HINT,
      r#"type Test = Uint8Array;"#: [
        {
          col: 12,
          fix: (FIX_DESC_ARRAY_BUFFER, "type Test = Uint8Array<ArrayBuffer>;"),
          fix: (FIX_DESC_ARRAY_BUFFER_LIKE, "type Test = Uint8Array<ArrayBufferLike>;"),
          fix: (FIX_DESC_SHARED_ARRAY_BUFFER, "type Test = Uint8Array<SharedArrayBuffer>;"),
        },
      ],
      r#"function test(): Uint8Array {}"#: [
        {
          col: 17,
          fix: (FIX_DESC_ARRAY_BUFFER, "function test(): Uint8Array<ArrayBuffer> {}"),
          fix: (FIX_DESC_ARRAY_BUFFER_LIKE, "function test(): Uint8Array<ArrayBufferLike> {}"),
          fix: (FIX_DESC_SHARED_ARRAY_BUFFER, "function test(): Uint8Array<SharedArrayBuffer> {}"),
        }
      ],
    };
  }
}
