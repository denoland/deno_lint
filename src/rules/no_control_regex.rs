#![allow(unused)]
// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{CallExpr, Expr, ExprOrSuper, Lit, NewExpr, Regex};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoControlRegex;

impl LintRule for NoControlRegex {
  fn new() -> Box<Self> {
    Box::new(NoControlRegex)
  }

  fn code(&self) -> &'static str {
    "no-regex-spaces"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoControlRegexVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoControlRegexVisitor {
  context: Context,
}

impl NoControlRegexVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_regex(&self, regex: &str) -> bool {
    dbg!(regex);
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

impl Visit for NoControlRegexVisitor {
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
    dbg!(new_expr);
    //if let Expr::Ident(ident) = &*new_expr.callee {
    //let name = ident.sym.to_string();
    //if name != "RegExp" {
    //return;
    //}
    //let scope = self.scope_manager.get_scope_for_span(new_expr.span);
    //if self.scope_manager.get_binding(scope, &ident.sym).is_some() {
    //return;
    //}
    //if let Some(args) = &new_expr.args {
    //if let Some(first_arg) = args.get(0) {
    //let regex_literal =
    //if let Expr::Lit(Lit::Str(literal)) = &*first_arg.expr {
    //&literal.value
    //} else if let Expr::Lit(Lit::Regex(regex)) = &*first_arg.expr {
    //&regex.exp
    //} else {
    //return;
    //};

    //if self.check_regex(regex_literal.as_ref()) {
    //self.context.add_diagnostic(
    //new_expr.span,
    //"no-regex-spaces",
    //"more than one consecutive spaces in RegExp is not allowed",
    //);
    //}
    //}
    //}
    //}
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    dbg!(call_expr);
    //if let exprorsuper::expr(expr) = &call_expr.callee {
    //if let expr::ident(ident) = expr.as_ref() {
    //let name = ident.sym.to_string();
    //if name != "regexp" {
    //return;
    //}
    //let scope = self.scope_manager.get_scope_for_span(call_expr.span);
    //if self.scope_manager.get_binding(scope, &ident.sym).is_some() {
    //return;
    //}

    //if !call_expr.args.is_empty() {
    //if let some(first_arg) = call_expr.args.get(0) {
    //let regex_literal =
    //if let expr::lit(lit::str(literal)) = &*first_arg.expr {
    //&literal.value
    //} else if let expr::lit(lit::regex(regex)) = &*first_arg.expr {
    //&regex.exp
    //} else {
    //return;
    //};

    //if self.check_regex(regex_literal.as_ref()) {
    //self.context.add_diagnostic(
    //call_expr.span,
    //"no-regex-spaces",
    //"more than one consecutive spaces in regexp is not allowed",
    //);
    //}
    //}
    //};
    //}
    //}
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_control_regex_valid() {
    assert_lint_ok_n::<NoControlRegex>(vec![
      r#"/x1f/"#,
      r#"/\\x1f/"#,
      r#"new RegExp('x1f')"#,
      r#"RegExp('x1f')"#,
      r#"new RegExp('[')"#,
      r#"RegExp('[')"#,
      r#"new (function foo(){})('\\x1f')"#,
    ]);
  }

  #[test]
  fn no_control_regex_invalid() {
    assert_lint_err::<NoControlRegex>(r#"/\x1f/"#, 10);
    assert_lint_err::<NoControlRegex>(r#"/\\\x1f\\x1e/"#, 10);
    assert_lint_err::<NoControlRegex>(r#"/\\\x1fFOO\\x00/"#, 10);
    assert_lint_err::<NoControlRegex>(r#"/FOO\\\x1fFOO\\x1f/"#, 10);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('\\x1f\\x1e')"#, 10);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('\\x1fFOO\\x00')"#, 10);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('FOO\\x1fFOO\\x1f')"#, 10);
    assert_lint_err::<NoControlRegex>(r#"RegExp('\\x1f')"#, 10);
    assert_lint_err::<NoControlRegex>(r#"/(?<a>\\x1f)/"#, 10);
  }
}
