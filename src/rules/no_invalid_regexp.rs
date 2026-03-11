// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;
use crate::{js_regex::*, tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;

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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoInvalidRegexpHandler {
      validator: EcmaRegexValidator::new(EcmaVersion::Es2022),
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

fn check_expr_for_string_literal(arg: &Argument) -> Option<String> {
  if let Argument::StringLiteral(s) = arg {
    return Some(s.value.to_string());
  }
  None
}

struct NoInvalidRegexpHandler {
  validator: EcmaRegexValidator,
}

impl NoInvalidRegexpHandler {
  fn handle_call_or_new_expr(
    &mut self,
    callee: &Expression,
    args: &[Argument],
    span: Span,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = callee {
      if ident.name.as_str() != "RegExp" || args.is_empty() {
        return;
      }
      if let Some(pattern) = check_expr_for_string_literal(&args[0]) {
        if args.len() > 1 {
          if let Some(flags) = check_expr_for_string_literal(&args[1]) {
            self.check_regex(&pattern, &flags, span, ctx);
            return;
          }
        }
        self.check_regex(&pattern, "", span, ctx);
      }
    }
  }

  fn check_regex(
    &mut self,
    pattern: &str,
    flags: &str,
    span: Span,
    ctx: &mut Context,
  ) {
    if self.check_for_invalid_flags(flags)
      || (!flags.is_empty()
        && self.check_for_invalid_pattern(pattern, flags.contains('u')))
      || (self.check_for_invalid_pattern(pattern, true)
        && self.check_for_invalid_pattern(pattern, false))
    {
      ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
    }
  }

  fn check_for_invalid_flags(&self, flags: &str) -> bool {
    self.validator.validate_flags(flags).is_err()
  }

  fn check_for_invalid_pattern(&mut self, source: &str, u_flag: bool) -> bool {
    self.validator.validate_pattern(source, u_flag).is_err()
  }
}

impl Handler<'_> for NoInvalidRegexpHandler {
  fn reg_exp_literal(
    &mut self,
    regex: &RegExpLiteral,
    ctx: &mut Context,
  ) {
    let pattern = regex.regex.pattern.text.as_str();
    let mut flags = String::new();
    let f = regex.regex.flags;
    if f.contains(RegExpFlags::G) { flags.push('g'); }
    if f.contains(RegExpFlags::I) { flags.push('i'); }
    if f.contains(RegExpFlags::M) { flags.push('m'); }
    if f.contains(RegExpFlags::S) { flags.push('s'); }
    if f.contains(RegExpFlags::U) { flags.push('u'); }
    if f.contains(RegExpFlags::Y) { flags.push('y'); }
    if f.contains(RegExpFlags::D) { flags.push('d'); }
    if f.contains(RegExpFlags::V) { flags.push('v'); }
    self.check_regex(pattern, &flags, regex.span, ctx);
  }

  fn call_expression(
    &mut self,
    call_expr: &CallExpression,
    ctx: &mut Context,
  ) {
    self.handle_call_or_new_expr(
      &call_expr.callee,
      &call_expr.arguments,
      call_expr.span,
      ctx,
    );
  }

  fn new_expression(
    &mut self,
    new_expr: &NewExpression,
    ctx: &mut Context,
  ) {
    self.handle_call_or_new_expr(
      &new_expr.callee,
      &new_expr.arguments,
      new_expr.span,
      ctx,
    );
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
