// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{Expr, Lit, NewExpr, Regex};
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
var re = new RegExp(/a {2}z/);
    "#,
      vec![NoRegexSpaces::new()],
      json!([]),
    );

    test_lint(
      "no_regex_spaces",
      r#"
var re = /a {30}z/;
  "#,
      vec![NoRegexSpaces::new()],
      json!([]),
    );
  }
}
