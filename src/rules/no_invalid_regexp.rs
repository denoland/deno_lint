// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::js_regex::*;
use swc_common::Span;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSpread;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoInvalidRegexp;

const CODE: &str = "no-invalid-regexp";
const MESSAGE: &str = "Invalid RegExp literal";
const HINT: &str = "Rework regular expression to be a valid";

impl LintRule for NoInvalidRegexp {
  fn new() -> Box<Self> {
    Box::new(NoInvalidRegexp)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoInvalidRegexpVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows specifying invalid regular expressions in RegExp constructors

Specifying an invalid regular expression literal will result in a SyntaxError at
compile time, however specifying an invalid regular expression string in the RegExp
constructor will only be discovered at runtime.
    
### Invalid:
```typescript
const invalidRegExp = new RegExp(')');
```

### Valid:
```typescript
const goodRegExp = new RegExp('.');
```
"#
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

struct NoInvalidRegexpVisitor<'c> {
  context: &'c mut Context,
  validator: EcmaRegexValidator,
}

impl<'c> NoInvalidRegexpVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
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
      if ident.sym != *"RegExp" || args.is_empty() {
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
      self
        .context
        .add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
    }
  }

  fn check_for_invalid_flags(&self, flags: &str) -> bool {
    self.validator.validate_flags(flags).is_err()
  }

  fn check_for_invalid_pattern(&mut self, source: &str, u_flag: bool) -> bool {
    self.validator.validate_pattern(source, u_flag).is_err()
  }
}

impl<'c> Visit for NoInvalidRegexpVisitor<'c> {
  noop_visit_type!();

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

  #[test]
  fn no_invalid_regexp_valid() {
    assert_lint_ok! {
      NoInvalidRegexp,
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
    };
  }

  #[test]
  fn no_invalid_regexp_invalid() {
    assert_lint_err! {
      NoInvalidRegexp,
      r#"RegExp('[');"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"RegExp('.', 'z');"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"new RegExp(')');"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"new RegExp('\\');"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"var foo = new RegExp('(', '');"#: [{ col: 10, message: MESSAGE, hint: HINT }],
      r#"/(?<a>a)\k</"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"/(?<!a){1}/"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"/(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\11/u"#: [{ col: 0, message: MESSAGE, hint: HINT }],
    }
  }
}
