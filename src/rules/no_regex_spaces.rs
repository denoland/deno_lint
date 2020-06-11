// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::{ScopeManager, ScopeVisitor};
use swc_ecma_ast::{CallExpr, Expr, ExprOrSuper, Lit, NewExpr, Regex};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoRegexSpaces;

impl LintRule for NoRegexSpaces {
  fn new() -> Box<Self> {
    Box::new(NoRegexSpaces)
  }

  fn code(&self) -> &'static str {
    "no-regex-spaces"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(&module, &module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = NoRegexSpacesVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoRegexSpacesVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl NoRegexSpacesVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }

  fn check_regex(&self, regex: &str) -> bool {
    lazy_static! {
      static ref DOUBLE_SPACE: regex::Regex =
        regex::Regex::new(r"(?u) {2}").unwrap();
      static ref BRACKETS: regex::Regex =
        regex::Regex::new(r"\[.*?[^\\]\]").unwrap();
      static ref SPACES: regex::Regex =
        regex::Regex::new(r#"(?u)( {2,})(?: [+*{?]|[^+*{?]|$)"#).unwrap();
    }
    if !DOUBLE_SPACE.is_match(regex) {
      return false;
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
        return true;
      }
    }
    false
  }
}

impl Visit for NoRegexSpacesVisitor {
  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {
    if self.check_regex(regex.exp.to_string().as_ref()) {
      self.context.add_diagnostic(
        regex.span,
        "no-regex-spaces",
        "more than one consecutive spaces in RegExp is not allowed",
      );
    }
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.to_string();
      if name != "RegExp" {
        return;
      }
      let scope = self.scope_manager.get_scope_for_span(new_expr.span);
      if self.scope_manager.get_binding(scope, &ident.sym).is_some() {
        return;
      }
      if let Some(args) = &new_expr.args {
        if let Some(first_arg) = args.get(0) {
          let regex_literal =
            if let Expr::Lit(Lit::Str(literal)) = &*first_arg.expr {
              &literal.value
            } else if let Expr::Lit(Lit::Regex(regex)) = &*first_arg.expr {
              &regex.exp
            } else {
              return;
            };

          if self.check_regex(regex_literal.as_ref()) {
            self.context.add_diagnostic(
              new_expr.span,
              "no-regex-spaces",
              "more than one consecutive spaces in RegExp is not allowed",
            );
          }
        }
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.to_string();
        if name != "RegExp" {
          return;
        }
        let scope = self.scope_manager.get_scope_for_span(call_expr.span);
        if self.scope_manager.get_binding(scope, &ident.sym).is_some() {
          return;
        }

        if !call_expr.args.is_empty() {
          if let Some(first_arg) = call_expr.args.get(0) {
            let regex_literal =
              if let Expr::Lit(Lit::Str(literal)) = &*first_arg.expr {
                &literal.value
              } else if let Expr::Lit(Lit::Regex(regex)) = &*first_arg.expr {
                &regex.exp
              } else {
                return;
              };

            if self.check_regex(regex_literal.as_ref()) {
              self.context.add_diagnostic(
                call_expr.span,
                "no-regex-spaces",
                "more than one consecutive spaces in RegExp is not allowed",
              );
            }
          }
        };
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_regex_spaces_valid() {
    assert_lint_ok_n::<NoRegexSpaces>(vec![
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
    ]);
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
