// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::extract_regex;
use std::iter::Peekable;
use std::str::Chars;
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSuper, NewExpr, Regex};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoControlRegex;

impl LintRule for NoControlRegex {
  fn new() -> Box<Self> {
    Box::new(NoControlRegex)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-control-regex"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoControlRegexVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use ascii control characters in regular expressions

Control characters are invisible characters in the ASCII range of 0-31.  It is
uncommon to use these in a regular expression and more often it is a mistake
in the regular expression.
    
### Invalid:
```typescript
// Examples using ASCII (31) Carriage Return (hex x0d)
const pattern1 = /\x0d/;
const pattern2 = /\u000d/;
const pattern3 = new RegExp("\\x0d");
const pattern4 = new RegExp("\\u000d");
```

### Valid:
```typescript
// Examples using ASCII (32) Space (hex x20)
const pattern1 = /\x20/;
const pattern2 = /\u0020/;
const pattern3 = new RegExp("\\x20");
const pattern4 = new RegExp("\\u0020");
```
"#
  }
}

struct NoControlRegexVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoControlRegexVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, cp: u64) {
    self.context.add_diagnostic_with_hint(
      span,
      "no-control-regex",
      format!(
        "Unexpected control character(s) in regular expression: \\x{:x}.",
        cp
      ),
      "Disable the rule if the control character (\\x... or \\u00..) was intentional, otherwise rework your RegExp",
    );
  }

  fn check_regex(&mut self, regex: &str, span: Span) {
    let mut iter = regex.chars().peekable();
    while let Some(ch) = iter.next() {
      if ch != '\\' {
        continue;
      }
      match iter.next() {
        Some('x') => {
          if let Some(cp) = read_hex_n(&mut iter, 2) {
            if cp <= 31 {
              self.add_diagnostic(span, cp);
              return;
            }
          }
        }
        Some('u') => {
          let cp = match iter.peek() {
            Some(&'{') => read_hex_until_brace(&mut iter),
            Some(_) => read_hex_n(&mut iter, 4),
            _ => None,
          };
          if let Some(cp) = cp {
            if cp <= 31 {
              self.add_diagnostic(span, cp);
              return;
            }
          }
        }
        _ => continue,
      }
    }
  }
}

/// Read the next n characters and try to parse it as hexadecimal.
fn read_hex_n(iter: &mut Peekable<Chars>, n: usize) -> Option<u64> {
  let mut s = String::new();
  for _ in 0..n {
    let ch = iter.next()?;
    s.push(ch);
  }
  u64::from_str_radix(s.as_str(), 16).ok()
}

/// Read characters until `}` and try to parse it as hexadecimal.
fn read_hex_until_brace(iter: &mut Peekable<Chars>) -> Option<u64> {
  iter.next(); // consume `{`
  let mut s = String::new();
  loop {
    let ch = iter.next()?;
    if ch == '}' {
      break;
    }
    s.push(ch);
  }
  u64::from_str_radix(s.as_str(), 16).ok()
}

impl<'c> Visit for NoControlRegexVisitor<'c> {
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
  fn no_control_regex_valid() {
    assert_lint_ok! {
      NoControlRegex,
      r#"/x1f/"#,
      r#"/\\x1f/"#,
      r#"/u001f/"#,
      r#"/\\u001f/"#,
      r#"/u{001f}/"#,
      r#"/\\u{001f}/"#,
      r#"/u{0001f}/"#,
      r#"/\\u{0001f}/"#,
      r#"new RegExp('x1f')"#,
      r#"RegExp('x1f')"#,
      r#"new RegExp('[')"#,
      r#"RegExp('[')"#,
      r#"new (function foo(){})('\\x1f')"#,
    };
  }

  #[test]
  fn no_control_regex_invalid() {
    assert_lint_err::<NoControlRegex>(r#"/\x1f/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/\u001f/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/\u{001f}/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/\u{0001f}/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/\\\x1f\\x1e/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/\\\x1fFOO\\x00/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"/FOO\\\x1fFOO\\x1f/"#, 0);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('\\x1f\\x1e')"#, 0);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('\\x1fFOO\\x00')"#, 0);
    assert_lint_err::<NoControlRegex>(r#"new RegExp('FOO\\x1fFOO\\x1f')"#, 0);
    assert_lint_err::<NoControlRegex>(r#"RegExp('\\x1f')"#, 0);
  }
}
