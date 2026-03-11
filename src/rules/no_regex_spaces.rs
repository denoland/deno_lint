// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::extract_regex;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct NoRegexSpaces;

const CODE: &str = "no-regex-spaces";
const MESSAGE: &str =
  "more than one consecutive spaces in RegExp is not allowed";

impl LintRule for NoRegexSpaces {
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
    let mut handler = NoRegexSpacesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoRegexSpacesHandler;

fn check_regex(regex: &str, span: Span, ctx: &mut Context) {
  static DOUBLE_SPACE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?u) {2}").unwrap());
  static BRACKETS: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\[.*?[^\\]\]").unwrap());
  static SPACES: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r#"(?u)( {2,})(?: [+*{?]|[^+*{?]|$)"#).unwrap()
  });

  if !DOUBLE_SPACE.is_match(regex) {
    return;
  }

  let mut character_classes = vec![];
  for mtch in BRACKETS.find_iter(regex) {
    character_classes.push((mtch.start(), mtch.end()));
  }

  for mtch in SPACES.find_iter(regex) {
    let not_in_classes = &character_classes
      .iter()
      .all(|v| mtch.start() < v.0 || v.1 <= mtch.start());
    if *not_in_classes {
      ctx.add_diagnostic(span, CODE, MESSAGE);
      return;
    }
  }
}

impl Handler<'_> for NoRegexSpacesHandler {
  fn reg_exp_literal(
    &mut self,
    regex: &RegExpLiteral,
    ctx: &mut Context,
  ) {
    check_regex(
      regex.regex.pattern.text.as_str(),
      regex.span,
      ctx,
    );
  }

  fn new_expression(
    &mut self,
    new_expr: &NewExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      if let Some(regex) =
        extract_regex(ctx.scoping(), ident, &new_expr.arguments)
      {
        check_regex(regex.as_str(), new_expr.span, ctx);
      }
    }
  }

  fn call_expression(
    &mut self,
    call_expr: &CallExpression,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = &call_expr.callee {
      if let Some(regex) =
        extract_regex(ctx.scoping(), ident, &call_expr.arguments)
      {
        check_regex(regex.as_str(), call_expr.span, ctx);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_regex_spaces_valid() {
    assert_lint_ok! {
      NoRegexSpaces,
      "var foo = /foo/;",
      "var foo = RegExp('foo')",
      "var foo = / /;",
      "var foo = RegExp(' ')",
      "var foo = / a b c d /;",
      "var foo = /bar {3}baz/g;",
      "var foo = RegExp('bar {3}baz', 'g')",
      "var foo = new RegExp('bar {3}baz')",
      "var foo = /bar\t\t\tbaz/;",
      "var foo = RegExp('bar\t\t\tbaz');",
      "var foo = new RegExp('bar\t\t\tbaz');",
      "var foo = /  +/;",
      "var foo = /  ?/;",
      "var foo = /  */;",
      "var foo = /  {2}/;",

      // don't report if RegExp shadowed
      "var RegExp = function() {}; var foo = new RegExp('bar   baz');",
      "var RegExp = function() {}; var foo = RegExp('bar   baz');",

      // don't report if there are no consecutive spaces in the source code
      r"var foo = /bar \\ baz/;",
      r"var foo = /bar\\ \\ baz/;",
      r"var foo = /bar \\u0020 baz/;",
      r"var foo = /bar\\u0020\\u0020baz/;",
      r"var foo = new RegExp('bar \\ baz')",
      r"var foo = new RegExp('bar\\ \\ baz')",
      r"var foo = new RegExp('bar \\\\ baz')",
      r"var foo = new RegExp('bar \\u0020 baz')",
      r"var foo = new RegExp('bar\\u0020\\u0020baz')",
      r"var foo = new RegExp('bar \\\\u0020 baz')",

      // don't report spaces in character classes
      "var foo = /[  ]/;",
      "var foo = /[   ]/;",
      "var foo = / [  ] /;",
      "var foo = / [  ] [  ] /;",
      "var foo = new RegExp('[  ]');",
      "var foo = new RegExp('[   ]');",
      "var foo = new RegExp(' [  ] ');",
      "var foo = RegExp(' [  ] [  ] ');",
      "var foo = new RegExp(' \\[   \\] ');",

      // TODO(@disizali) invalid regexes must handled on separated rule called `no-invalid-regexp`.
      // "var foo = new RegExp('[  ');",
      // "var foo = new RegExp('{  ', 'u');",
      // "var foo = new RegExp(' \\[   ');",
    };
  }

  #[test]
  fn no_regex_spaces_invalid() {
    assert_lint_err! {
      NoRegexSpaces,
      "let foo = /bar  baz/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /bar    baz/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = / a b  c d /;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp(' a b c d  ');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp('bar    baz');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = new RegExp('bar    baz');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "{ let RegExp = function() {}; } var foo = RegExp('bar    baz');": [
        {
          col: 42,
          message: MESSAGE,
        }
      ],
      "let foo = /bar   {3}baz/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /bar    ?baz/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp('bar   +baz')": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = new RegExp('bar    ');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /bar\\  baz/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /[   ]  /;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /  [   ] /;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = new RegExp('[   ]  ');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp('  [ ]');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /\\[  /;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /\\[  \\]/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /(?:  )/;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp('^foo(?=   )');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /\\  /": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = / \\  /": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = /  foo   /;": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = new RegExp('\\\\d  ')": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = RegExp('\\u0041   ')": [
        {
          col: 10,
          message: MESSAGE,
        }
      ],
      "let foo = new RegExp('\\\\[  \\\\]');": [
        {
          col: 10,
          message: MESSAGE,
        }
      ]
    };
  }
}
