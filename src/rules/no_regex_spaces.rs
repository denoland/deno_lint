// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::extract_regex;
use once_cell::sync::Lazy;
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSuper, NewExpr, Regex};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoRegexSpaces;

impl LintRule for NoRegexSpaces {
  fn new() -> Box<Self> {
    Box::new(NoRegexSpaces)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-regex-spaces"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoRegexSpacesVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoRegexSpacesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoRegexSpacesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
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
        .all(|ref v| mtch.start() < v.0 || v.1 <= mtch.start());
      if *not_in_classes {
        self.context.add_diagnostic(
          span,
          "no-regex-spaces",
          "more than one consecutive spaces in RegExp is not allowed",
        );
        return;
      }
    }
  }
}

impl<'c> Visit for NoRegexSpacesVisitor<'c> {
  noop_visit_type!();

  fn visit_regex(&mut self, regex: &Regex, parent: &dyn Node) {
    self.check_regex(regex.exp.to_string().as_str(), regex.span);
    swc_ecmascript::visit::visit_regex(self, regex, parent);
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if let Some(args) = &new_expr.args {
        if let Some(regex) = extract_regex(&self.context.scope, ident, args) {
          self.check_regex(regex.as_str(), new_expr.span);
        }
      }
    }
    swc_ecmascript::visit::visit_new_expr(self, new_expr, parent);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        if let Some(regex) =
          extract_regex(&self.context.scope, ident, &call_expr.args)
        {
          self.check_regex(regex.as_str(), call_expr.span);
        }
      }
    }
    swc_ecmascript::visit::visit_call_expr(self, call_expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
    assert_lint_err::<NoRegexSpaces>("let foo = /bar  baz/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /bar    baz/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = / a b  c d /;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp(' a b c d  ');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp('bar    baz');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = new RegExp('bar    baz');", 10);
    assert_lint_err::<NoRegexSpaces>(
      "{ let RegExp = function() {}; } var foo = RegExp('bar    baz');",
      42,
    );
    assert_lint_err::<NoRegexSpaces>("let foo = /bar   {3}baz/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /bar    ?baz/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp('bar   +baz')", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = new RegExp('bar    ');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /bar\\  baz/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /[   ]  /;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /  [   ] /;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = new RegExp('[   ]  ');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp('  [ ]');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /\\[  /;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /\\[  \\]/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /(?:  )/;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp('^foo(?=   )');", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /\\  /", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = / \\  /", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = /  foo   /;", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = new RegExp('\\\\d  ')", 10);
    assert_lint_err::<NoRegexSpaces>("let foo = RegExp('\\u0041   ')", 10);
    assert_lint_err::<NoRegexSpaces>(
      "let foo = new RegExp('\\\\[  \\\\]');",
      10,
    );
  }
}
