// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::swc_util::extract_regex;
use once_cell::sync::Lazy;
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSuper, NewExpr, Regex};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoRegexSpaces;

const CODE: &str = "no-regex-spaces";
const MESSAGE: &str =
  "more than one consecutive spaces in RegExp is not allowed";

impl LintRule for NoRegexSpaces {
  fn new() -> Box<Self> {
    Box::new(NoRegexSpaces)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoRegexSpacesVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_regex_spaces.md")
  }
}

struct NoRegexSpacesVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoRegexSpacesVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_regex(&mut self, regex: &str, span: Span) {
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
        self.context.add_diagnostic(span, CODE, MESSAGE);
        return;
      }
    }
  }
}

impl<'c, 'view> VisitAll for NoRegexSpacesVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_regex(&mut self, regex: &Regex, _: &dyn Node) {
    self.check_regex(regex.exp.to_string().as_str(), regex.span);
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if let Some(args) = &new_expr.args {
        if let Some(regex) = extract_regex(self.context.scope(), ident, args) {
          self.check_regex(regex.as_str(), new_expr.span);
        }
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        if let Some(regex) =
          extract_regex(self.context.scope(), ident, &call_expr.args)
        {
          self.check_regex(regex.as_str(), call_expr.span);
        }
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
