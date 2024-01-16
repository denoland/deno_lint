// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::swc::parser::token::{IdentLike, KnownIdent, Token, Word};
use deno_ast::view as ast_view;
use deno_ast::{SourceRanged, SourceRangedForSpanned};
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoImportAssertions;

const CODE: &str = "no-import-assertions";
const MESSAGE: &str =
  "The `assert` keyword is deprecated for import attributes";
const HINT: &str = "Instead use the `with` keyword";

impl LintRule for NoImportAssertions {
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
    NoImportAssertionsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_import_assertions.md")
  }
}

struct NoImportAssertionsHandler;

impl Handler for NoImportAssertionsHandler {
  fn import_decl(
    &mut self,
    import_decl: &ast_view::ImportDecl,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let Some(with) = import_decl.with;
      if let Some(prev_token_and_span) = with.start().previous_token_fast(ctx.program());
      if let Token::Word(word) = &prev_token_and_span.token;
      if let Word::Ident(ident_like) = word;
      if let IdentLike::Known(known_ident) = ident_like;
      if matches!(known_ident, KnownIdent::Assert);
      then {
        ctx.add_diagnostic_with_hint(
          prev_token_and_span.span.range(),
          CODE,
          MESSAGE,
          HINT,
        );
      }
    }
  }

  fn call_expr(&mut self, call_expr: &ast_view::CallExpr, ctx: &mut Context) {
    if_chain! {
      if matches!(call_expr.callee, ast_view::Callee::Import(_));
      if let Some(expr_or_spread) = call_expr.args.get(1);
      if let ast_view::Expr::Object(object_lit) = expr_or_spread.expr;
      then {
        for prop_or_spread in object_lit.props.iter() {
          if_chain! {
            if let ast_view::PropOrSpread::Prop(prop) = prop_or_spread;
            if let ast_view::Prop::KeyValue(key_value_prop) = prop;
            then {
              match key_value_prop.key {
                ast_view::PropName::Ident(ident) => {
                  if ident.sym().as_ref() == "assert" {
                    ctx.add_diagnostic_with_hint(
                      ident.range(),
                      CODE,
                      MESSAGE,
                      HINT,
                    );
                  }
                },
                ast_view::PropName::Str(str) => {
                  if str.value().as_ref() == "assert" {
                    ctx.add_diagnostic_with_hint(
                      str.range(),
                      CODE,
                      MESSAGE,
                      HINT,
                    );
                  }
                }
                _ => (),
              }
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_import_assertions_valid() {
    assert_lint_ok! {
      NoImportAssertions,
      r#"import foo from './foo.js';"#,
      r#"import foo from './foo.js' with { bar: 'bar' };"#,
      r#"import('./foo.js');"#,
      r#"import('./foo.js', { with: { bar: 'bar' } });"#,
      r#"import('./foo.js', { "with": { bar: 'bar' } });"#,
    };
  }

  #[test]
  fn no_import_assertions_invalid() {
    assert_lint_err! {
      NoImportAssertions,
      MESSAGE,
      HINT,
      r#"import foo from './foo.js' assert { bar: 'bar' };"#: [
        {
          line: 1,
          col: 27,
        },
      ],
      r#"import('./foo.js', { assert: { bar: 'bar' } });"#: [
        {
          line: 1,
          col: 21,
        },
      ],
      r#"import('./foo.js', { "assert": { bar: 'bar' } });"#: [
        {
          line: 1,
          col: 21,
        },
      ],
    };
  }
}
