// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::{ScopeManager, ScopeVisitor};
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{CallExpr, Expr, ExprOrSuper, NewExpr, Regex};
use crate::swc_util::extract_regex;
use std::iter::Peekable;
use std::str::Chars;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoControlRegex;

impl LintRule for NoControlRegex {
  fn new() -> Box<Self> {
    Box::new(NoControlRegex)
  }

  fn code(&self) -> &'static str {
    "no-control-regex"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(&module, &module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = NoControlRegexVisitor::new(context, scope_manager);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoControlRegexVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl NoControlRegexVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }

  fn add_diagnostic(&self, span: Span, cp: u64) {
    self.context.add_diagnostic(
      span,
      "no-control-regex",
      &format!(
        "Unexpected control character(s) in regular expression: \\x{:x}.",
        cp
      ),
    );
  }

  fn check_regex(&self, regex: &str, span: Span) {
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

impl Visit for NoControlRegexVisitor {
  fn visit_regex(&mut self, regex: &Regex, parent: &dyn Node) {
    self.check_regex(regex.exp.to_string().as_str(), regex.span);
    swc_ecma_visit::visit_regex(self, regex, parent);
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      if let Some(args) = &new_expr.args {
        if let Some(regex) =
          extract_regex(&self.scope_manager, new_expr.span, ident, args)
        {
          self.check_regex(regex.as_str(), new_expr.span);
        }
      }
    }
    swc_ecma_visit::visit_new_expr(self, new_expr, parent);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        if let Some(regex) = extract_regex(
          &self.scope_manager,
          call_expr.span,
          ident,
          &call_expr.args,
        ) {
          self.check_regex(regex.as_str(), call_expr.span);
        }
      }
    }
    swc_ecma_visit::visit_call_expr(self, call_expr, parent);
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
    ]);
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
