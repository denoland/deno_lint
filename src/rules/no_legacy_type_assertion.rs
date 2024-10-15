// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::{view as ast_view, MediaType, SourceRanged};

#[derive(Debug)]
pub struct NoLegacyTypeAssertion;

const CODE: &str = "no-legacy-type-assertion";
const MESSAGE: &str =
  "TypeScript's `<Type> value` type assertion syntax is discouraged to use";
const HINT: &str = "Use `as` assertion syntax instead";
const FIX_DESC: &str = "Convert to `as` assertion";

impl LintRule for NoLegacyTypeAssertion {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    // The syntax can't appear in js, or jsx/tsx, or d.ts files.
    if matches!(
      context.media_type(),
      MediaType::TypeScript | MediaType::Mts | MediaType::Cts
    ) {
      NoLegacyTypeAssertionHandler.traverse(program, context);
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_legacy_type_assertion.md")
  }
}

struct NoLegacyTypeAssertionHandler;

impl Handler for NoLegacyTypeAssertionHandler {
  fn ts_type_assertion(
    &mut self,
    n: &ast_view::TsTypeAssertion,
    ctx: &mut Context,
  ) {
    let expr = n.expr.text_fast(ctx.parsed_source());
    let typ = n.type_ann.text_fast(ctx.parsed_source());
    let fixed = format!("({} as {})", expr, typ);
    ctx.add_diagnostic_with_fixes(
      n.range(),
      CODE,
      MESSAGE,
      Some(HINT.to_owned()),
      vec![LintFix {
        changes: vec![LintFixChange {
          new_text: Cow::Owned(fixed),
          range: n.range(),
        }],
        description: Cow::Borrowed(FIX_DESC),
      }],
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_legacy_type_assertion_valid() {
    assert_lint_ok! {
      NoLegacyTypeAssertion,
      filename: "file:///foo.ts",

      r#"const a = 1 as number"#,
    };
  }

  #[test]
  fn no_legacy_type_assertion_invalid() {
    assert_lint_err! {
      NoLegacyTypeAssertion,
      "<number> 1": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Convert to `as` assertion",
            "(1 as number)"
          )
        },
      ],
      "const x = 5 + <number> 5;": [
        {
          col: 14,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Convert to `as` assertion",
            "const x = 5 + (5 as number);"
          )
        }
      ],
    };
  }
}
