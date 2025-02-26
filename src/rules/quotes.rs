// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
//
// Enforces consistent use of quotes.
// Similar to ESLint's quotes rule.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tag;
use crate::Program;
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct Quotes;

const CODE: &str = "quotes";
const MESSAGE: &str = "Strings must use double quotes";
const HINT: &str = "Replace single quotes with double quotes";

impl LintRule for Quotes {
  fn tags(&self) -> &'static [Tag] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    QuotesHandler.traverse(program, context);
  }
}

struct QuotesHandler;

impl Handler for QuotesHandler {
  fn str(&mut self, lit: &ast_view::Str, ctx: &mut Context) {
    let raw_str = lit.range().text_fast(ctx.text_info());
    
    // Skip if template literal
    if raw_str.starts_with('`') {
      return;
    }
    
    // Check if using single quotes
    if raw_str.starts_with('\'') {
      ctx.add_diagnostic_with_hint(lit.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn quotes_valid() {
    assert_lint_ok! {
        Quotes,
        r#"var foo = "bar";"#,
        r#"var foo = 1;"#,
        r#"var foo = `bar`;"#,
        r#"var foo = `bar${baz}`;"#,
        r#"var foo = "don't";"#,
        r#"var obj = {"key0": 0, "key1": 1};"#,
        r#"class Foo { "bar"(){} }"#,
        r#"class C { "f"; "m"() {} }"#,
        // Template literals with substitutions or line breaks
        r#"var foo = `back${x}tick`;"#,
        r#"var foo = `back
tick`;"#,
        r#"var foo = tag`backtick`;"#,
        // Directives
        r#""use strict"; var foo = "bar";"#,
        // Import/Export (would need module parsing)
        // JSX (would need JSX syntax)
    };
  }

  #[test]
  fn quotes_invalid() {
    assert_lint_err! {
        Quotes,
        r#"var foo = 'bar';"#: [{
            col: 10,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"var foo = 'don\'t';"#: [{
            col: 10,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"var msg = 'Plugin ' + name + ' not found'"#: [{
            col: 10,
            message: MESSAGE,
            hint: HINT,
        }, {
            col: 29,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"class C { 'foo'; }"#: [{
            col: 10,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"class C { 'foo'() {} }"#: [{
            col: 10,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"var obj = {['key1']: 1};"#: [{
            col: 12,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    // Directives and special contexts
    assert_lint_err! {
        Quotes,
        r#"{ 'use strict'; }"#: [{
            col: 2,
            message: MESSAGE,
            hint: HINT,
        }]
    };

    assert_lint_err! {
        Quotes,
        r#"('foo'); 'bar';"#: [{
            col: 1,
            message: MESSAGE,
            hint: HINT,
        }, {
            col: 9,
            message: MESSAGE,
            hint: HINT,
        }]
    };
  }
}