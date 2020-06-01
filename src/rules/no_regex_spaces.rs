// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{CallExpr, Expr, ExprOrSuper, Lit, NewExpr, Regex};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoRegexSpaces;

impl LintRule for NoRegexSpaces {
  fn new() -> Box<Self> {
    Box::new(NoRegexSpaces)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoRegexSpacesVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoRegexSpacesVisitor {
  context: Context,
}

impl NoRegexSpacesVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoRegexSpacesVisitor {
  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {
    if regex.exp.to_string().matches("  ").count() > 0 {
      self.context.add_diagnostic(
        regex.span,
        "noRegexSpaces",
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

          if regex_literal.matches("  ").count() > 0 {
            self.context.add_diagnostic(
              new_expr.span,
              "noRegexSpaces",
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
        } else {
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

              if regex_literal.matches("  ").count() > 0 {
                self.context.add_diagnostic(
                  call_expr.span,
                  "noRegexSpaces",
                  "more than one consecutive spaces in RegExp is not allowed",
                );
              }
            }
          };
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn no_regex_spaces_test() {
    test_lint(
      "no_regex_spaces",
      r#"
var re = /a   z/;
      "#,
      vec![NoRegexSpaces::new()],
      json!([{
        "code": "noRegexSpaces",
        "message": "more than one consecutive spaces in RegExp is not allowed",
        "location": {
          "filename": "no_regex_spaces",
          "line": 2,
          "col": 9,
        }
      }]),
    );

    test_lint(
      "no_regex_spaces",
      r#"
var re = new RegExp("a   z");
      "#,
      vec![NoRegexSpaces::new()],
      json!([{
        "code": "noRegexSpaces",
        "message": "more than one consecutive spaces in RegExp is not allowed",
        "location": {
          "filename": "no_regex_spaces",
          "line": 2,
          "col": 9,
        }
      }]),
    );

    test_lint(
      "no_regex_spaces",
      r#"
var re = new RegExp(/a  z/);
    "#,
      vec![NoRegexSpaces::new()],
      json!([{
        "code": "noRegexSpaces",
        "message": "more than one consecutive spaces in RegExp is not allowed",
        "location": {
          "filename": "no_regex_spaces",
          "line": 2,
          "col": 9,
        }
      }]),
    );

    test_lint(
      "no_regex_spaces",
      r#"
var re = new RegExp(/a  z/);
    "#,
      vec![NoRegexSpaces::new()],
      json!([{
        "code": "noRegexSpaces",
        "message": "more than one consecutive spaces in RegExp is not allowed",
        "location": {
          "filename": "no_regex_spaces",
          "line": 2,
          "col": 9,
        }
      }]),
    );
  }

  #[test]
  fn no_regex_spaces_ok() {
    test_lint(
      "no_regex_spaces",
      r#"
          var foo = /foo/;
          var foo = RegExp('foo')
          var foo = / /;
          var foo = RegExp(' ')
          var foo = / a b c d /;
          var foo = /bar {3}baz/g;
          var foo = RegExp('bar {3}baz','g')
          var foo = new RegExp('bar {3}baz')
          var foo = /bar\t\t\tbaz/;
          var foo = RegExp('bar\t\t\tbaz');
          var foo = new RegExp('bar\t\t\tbaz');
          var RegExp = function() {}; var foo = new RegExp('bar   baz');
          var RegExp = function() {}; var foo = RegExp('bar   baz');
          var foo = /  +/;
          var foo = /  ?/;
          var foo = /  */;
          var foo = /  {2}/;
          var foo = /bar \\ baz/;
          var foo = /bar\\ \\ baz/;
          var foo = /bar \\u0020 baz/;
          var foo = /bar\\u0020\\u0020baz/;
          var foo = new RegExp('bar \\ baz')
          var foo = new RegExp('bar\\ \\ baz')
          var foo = new RegExp('bar \\\\ baz')
          var foo = new RegExp('bar \\u0020 baz')
          var foo = new RegExp('bar\\u0020\\u0020baz')
          var foo = new RegExp('bar \\\\u0020 baz')
          /*
          var foo = /[  ]/;
          var foo = /[   ]/;
          var foo = / [  ] /;
          var foo = / [  ] [  ] /;
          var foo = new RegExp('[  ]');
          var foo = new RegExp('[   ]');
          var foo = new RegExp(' [  ] ');
          var foo = RegExp(' [  ] [  ] ');
          var foo = new RegExp(' \\[   ');
          var foo = new RegExp(' \\[   \\] ');
          var foo = new RegExp('[  ');
          var foo = new RegExp('{  ','u');
          */
        "#,
      vec![NoRegexSpaces::new()],
      json!([]),
    );
  }
}
