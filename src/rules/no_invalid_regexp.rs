// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::tags::Tags;
use crate::Program;
use crate::ProgramRef;
use crate::{js_regex::*, tags};
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ExprOrSpread;
use deno_ast::swc::ecma_visit::noop_visit_type;
use deno_ast::swc::ecma_visit::Visit;
use deno_ast::SourceRange;
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct NoInvalidRegexp;

const CODE: &str = "no-invalid-regexp";
const MESSAGE: &str = "Invalid RegExp literal";
const HINT: &str = "Rework regular expression to be a valid";

impl LintRule for NoInvalidRegexp {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoInvalidRegexpVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
  }
}

fn check_expr_for_string_literal(expr: &Expr) -> Option<String> {
  if let Expr::Lit(deno_ast::swc::ast::Lit::Str(pattern_string)) = expr {
    let s: &str = &pattern_string.value;
    return Some(s.to_owned());
  }
  None
}

struct NoInvalidRegexpVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  validator: EcmaRegexValidator,
}

impl<'c, 'view> NoInvalidRegexpVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self {
      context,
      validator: EcmaRegexValidator::new(EcmaVersion::Es2022),
    }
  }

  fn handle_call_or_new_expr(
    &mut self,
    callee: &Expr,
    args: &[ExprOrSpread],
    range: SourceRange,
  ) {
    if let Expr::Ident(ident) = callee {
      if ident.sym != *"RegExp" || args.is_empty() {
        return;
      }
      if let Some(pattern) = &check_expr_for_string_literal(&args[0].expr) {
        if args.len() > 1 {
          if let Some(flags) = &check_expr_for_string_literal(&args[1].expr) {
            self.check_regex(pattern, flags, range);
            return;
          }
        }
        self.check_regex(pattern, "", range);
      }
    }
  }

  fn check_regex(&mut self, pattern: &str, flags: &str, range: SourceRange) {
    if self.check_for_invalid_flags(flags)
      || (!flags.is_empty()
        && self.check_for_invalid_pattern(pattern, flags.contains('u')))
      || (self.check_for_invalid_pattern(pattern, true)
        && self.check_for_invalid_pattern(pattern, false))
    {
      self
        .context
        .add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
    }
  }

  fn check_for_invalid_flags(&self, flags: &str) -> bool {
    self.validator.validate_flags(flags).is_err()
  }

  fn check_for_invalid_pattern(&mut self, source: &str, u_flag: bool) -> bool {
    self.validator.validate_pattern(source, u_flag).is_err()
  }
}

impl Visit for NoInvalidRegexpVisitor<'_, '_> {
  noop_visit_type!();

  fn visit_regex(&mut self, regex: &deno_ast::swc::ast::Regex) {
    self.check_regex(&regex.exp, &regex.flags, regex.range());
  }

  fn visit_call_expr(&mut self, call_expr: &deno_ast::swc::ast::CallExpr) {
    if let deno_ast::swc::ast::Callee::Expr(expr) = &call_expr.callee {
      self.handle_call_or_new_expr(expr, &call_expr.args, call_expr.range());
    }
  }

  fn visit_new_expr(&mut self, new_expr: &deno_ast::swc::ast::NewExpr) {
    if new_expr.args.is_some() {
      self.handle_call_or_new_expr(
        &new_expr.callee,
        new_expr.args.as_ref().unwrap(),
        new_expr.range(),
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_invalid_regexp_valid() {
    assert_lint_ok! {
      NoInvalidRegexp,
      r"RegExp('');
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
let re = new RegExp('foo', x);",
    };
  }

  #[test]
  fn no_invalid_regexp_invalid() {
    assert_lint_err! {
      NoInvalidRegexp,
      r"RegExp('[');": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"RegExp('.', 'z');": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"new RegExp(')');": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"new RegExp('\\');": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"var foo = new RegExp('(', '');": [{ col: 10, message: MESSAGE, hint: HINT }],
      r"/(?<a>a)\k</": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"/(?<!a){1}/": [{ col: 0, message: MESSAGE, hint: HINT }],
      r"/(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\11/u": [{ col: 0, message: MESSAGE, hint: HINT }],
    }
  }
}
