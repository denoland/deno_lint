// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::js_regex::*;
use swc_atoms::JsWord;
use swc_common::Span;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSpread;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoInvalidRegexp;

impl LintRule for NoInvalidRegexp {
  fn new() -> Box<Self> {
    Box::new(NoInvalidRegexp)
  }

  fn code(&self) -> &'static str {
    "no-invalid-regexp"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoInvalidRegexpVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

fn check_expr_for_string_literal(expr: &Expr) -> Option<String> {
  if let Expr::Lit(lit_expr) = expr {
    if let swc_ecmascript::ast::Lit::Str(pattern_string) = lit_expr {
      let s: &str = &pattern_string.value;
      return Some(s.to_owned());
    }
  }
  None
}

struct NoInvalidRegexpVisitor {
  context: Arc<Context>,
  validator: EcmaRegexValidator,
}

impl NoInvalidRegexpVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self {
      context,
      validator: EcmaRegexValidator::new(EcmaVersion::ES2018),
    }
  }

  fn handle_call_or_new_expr(
    &mut self,
    callee: &Expr,
    args: &[ExprOrSpread],
    span: Span,
  ) {
    if let Expr::Ident(ident) = callee {
      if ident.sym != JsWord::from("RegExp") || args.is_empty() {
        return;
      }
      if let Some(pattern) = &check_expr_for_string_literal(&*args[0].expr) {
        if args.len() > 1 {
          if let Some(flags) = &check_expr_for_string_literal(&*args[1].expr) {
            self.check_regex(pattern, flags, span);
            return;
          }
        }
        self.check_regex(pattern, "", span);
      }
    }
  }

  fn check_regex(&mut self, pattern: &str, flags: &str, span: Span) {
    if self.check_for_invalid_flags(flags)
      || (!flags.is_empty()
        && self.check_for_invalid_pattern(pattern, flags.contains('u')))
      || (self.check_for_invalid_pattern(pattern, true)
        && self.check_for_invalid_pattern(pattern, false))
    {
      self.context.add_diagnostic(
        span,
        "no-invalid-regexp",
        "Invalid RegExp literal",
      );
    }
  }

  fn check_for_invalid_flags(&self, flags: &str) -> bool {
    self.validator.validate_flags(flags).is_err()
  }

  fn check_for_invalid_pattern(&mut self, source: &str, u_flag: bool) -> bool {
    self.validator.validate_pattern(source, u_flag).is_err()
  }
}

impl Visit for NoInvalidRegexpVisitor {
  fn visit_regex(
    &mut self,
    regex: &swc_ecmascript::ast::Regex,
    _parent: &dyn Node,
  ) {
    self.check_regex(&regex.exp, &regex.flags, regex.span);
  }

  fn visit_call_expr(
    &mut self,
    call_expr: &swc_ecmascript::ast::CallExpr,
    _paren: &dyn Node,
  ) {
    if let swc_ecmascript::ast::ExprOrSuper::Expr(expr) = &call_expr.callee {
      self.handle_call_or_new_expr(&*expr, &call_expr.args, call_expr.span);
    }
  }

  fn visit_new_expr(
    &mut self,
    new_expr: &swc_ecmascript::ast::NewExpr,
    _parent: &dyn Node,
  ) {
    if new_expr.args.is_some() {
      self.handle_call_or_new_expr(
        &*new_expr.callee,
        new_expr.args.as_ref().unwrap(),
        new_expr.span,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_invalid_regexp_valid() {
    assert_lint_ok::<NoInvalidRegexp>(
      r#"RegExp('');
RegExp();
RegExp('.', 'g');
new RegExp('.');
new RegExp;
new RegExp('.', 'im');
new RegExp('.', y);
global.RegExp('\\');

new RegExp('.', 'y');
new RegExp('.', 'u');
new RegExp('.', 'yu');
new RegExp('/', 'yu');
new RegExp('\/', 'yu');
new RegExp('.', 'y');
new RegExp('.', 'u');
new RegExp('.', 'yu');
new RegExp('/', 'yu');
new RegExp('\/', 'yu');
new RegExp('\\u{65}', 'u');
new RegExp('[\\u{0}-\\u{1F}]', 'u');
new RegExp('.', 's');
new RegExp('(?<=a)b');
new RegExp('(?<!a)b');
new RegExp('(?<a>b)\\k<a>');
new RegExp('(?<a>b)\\k<a>', 'u');
new RegExp('\\p{Letter}', 'u');

var foo = new RegExp('(a)bc[de]', '');
var foo = new RegExp('a', '');
/(a)bc[de]/.test('abcd');
/(a)bc[de]/u;
let x = new FooBar('\\');
let re = new RegExp('foo', x);"#,
    );
  }

  #[test]
  fn no_invalid_regexp_invalid() {
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"RegExp('[');"#, 1, 0);
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"RegExp('.', 'z');"#, 1, 0);
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"new RegExp(')');"#, 1, 0);
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"new RegExp('\\');"#, 1, 0);

    assert_lint_err_on_line::<NoInvalidRegexp>(
      r#"var foo = new RegExp('(', '');"#,
      1,
      10,
    );
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"/(?<a>a)\k</"#, 1, 0);
    assert_lint_err_on_line::<NoInvalidRegexp>(r#"/(?<!a){1}/"#, 1, 0);
    assert_lint_err_on_line::<NoInvalidRegexp>(
      r#"/(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\11/u"#,
      1,
      0,
    );
  }
}
