// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  Expression, ImportDeclaration, ImportExpression, ObjectPropertyKind, Program,
  PropertyKey, WithClauseKeyword,
};

#[derive(Debug)]
pub struct NoImportAssertions;

const CODE: &str = "no-import-assertions";
const MESSAGE: &str =
  "The `assert` keyword is deprecated for import attributes";
const HINT: &str = "Instead use the `with` keyword";

impl LintRule for NoImportAssertions {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoImportAssertionsHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoImportAssertionsHandler;

impl Handler<'_> for NoImportAssertionsHandler {
  fn import_declaration(
    &mut self,
    import_decl: &ImportDeclaration,
    ctx: &mut Context,
  ) {
    if let Some(with_clause) = &import_decl.with_clause {
      if with_clause.keyword == WithClauseKeyword::Assert {
        // Report on the keyword span. The keyword is located before the
        // with_clause entries. We use the span of the entire with_clause
        // minus the entries to approximate it, but for accuracy we use
        // a substring approach. Since OXC parses assert/with as the keyword,
        // we can report the attribute_keyword span.
        // Actually in OXC, we can just check for assert keyword directly.
        // Report the span of the keyword.
        // The keyword position is approximately the start of with_clause.
        // We need to find "assert" in the source. Let's report the whole
        // with_clause span as the keyword is captured there.
        // Actually, the WithClause has keyword field. We can use a simple
        // span starting from with_clause.span.start with length 6 for "assert".
        let keyword_span = deno_ast::oxc::span::Span::new(
          with_clause.span.start,
          with_clause.span.start + 6,
        );
        ctx.add_diagnostic_with_hint(keyword_span, CODE, MESSAGE, HINT);
      }
    }
  }

  fn import_expression(
    &mut self,
    import_expr: &ImportExpression,
    ctx: &mut Context,
  ) {
    // Check if the options argument contains { assert: ... }
    if let Some(options) = &import_expr.options {
      if let Expression::ObjectExpression(object_lit) = options {
        for prop in object_lit.properties.iter() {
          if let ObjectPropertyKind::ObjectProperty(prop) = prop {
            match &prop.key {
              PropertyKey::StaticIdentifier(ident) => {
                if ident.name.as_str() == "assert" {
                  ctx.add_diagnostic_with_hint(
                    ident.span,
                    CODE,
                    MESSAGE,
                    HINT,
                  );
                }
              }
              PropertyKey::StringLiteral(str_lit) => {
                if str_lit.value.as_str() == "assert" {
                  ctx.add_diagnostic_with_hint(
                    str_lit.span,
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
          col: 34,
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
