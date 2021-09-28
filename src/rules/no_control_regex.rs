// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::extract_regex;
use crate::{Program, ProgramRef};
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::{CallExpr, Expr, ExprOrSuper, NewExpr, Regex};
use derive_more::Display;
use std::iter::Peekable;
use std::str::Chars;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoControlRegex;

const CODE: &str = "no-control-regex";

#[derive(Display)]
enum NoControlRegexMessage {
  #[display(
    fmt = "Unexpected control character(s) in regular expression: \\x{:x}.",
    _0
  )]
  Unexpected(u64),
}

#[derive(Display)]
enum NoControlRegexHint {
  #[display(
    fmt = "Disable the rule if the control character (\\x... or \\u00..) was intentional, otherwise rework your RegExp"
  )]
  DisableOrRework,
}

impl LintRule for NoControlRegex {
  fn new() -> Arc<Self> {
    Arc::new(NoControlRegex)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoControlRegexHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_control_regex.md")
  }
}

struct NoControlRegexHandler;

fn add_diagnostic(span: Span, cp: u64, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    span,
    CODE,
    NoControlRegexMessage::Unexpected(cp),
    NoControlRegexHint::DisableOrRework,
  );
}

fn check_regex(regex: &str, span: Span, ctx: &mut Context) {
  let mut iter = regex.chars().peekable();
  while let Some(ch) = iter.next() {
    if ch != '\\' {
      continue;
    }
    match iter.next() {
      Some('x') => {
        if let Some(cp) = read_hex_n(&mut iter, 2) {
          if cp <= 31 {
            add_diagnostic(span, cp, ctx);
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
            add_diagnostic(span, cp, ctx);
            return;
          }
        }
      }
      _ => continue,
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

impl Handler for NoControlRegexHandler {
  fn regex(&mut self, regex: &Regex, ctx: &mut Context) {
    check_regex(regex.inner.exp.to_string().as_str(), regex.span(), ctx);
  }

  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    if let Expr::Ident(ident) = new_expr.callee {
      if let Some(args) = &new_expr.args {
        if let Some(regex) = extract_regex(ctx.scope(), ident, args) {
          check_regex(regex.as_str(), new_expr.span(), ctx);
        }
      }
    }
  }

  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr {
        if let Some(regex) = extract_regex(ctx.scope(), ident, &call_expr.args)
        {
          check_regex(regex.as_str(), call_expr.span(), ctx);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read_hex_n() {
    let tests = [
      (r#"1f"#, Some(0x1f)),
      (r#"001f"#, Some(0x1f)),
      (r#"1g"#, None),
      (r#"001g"#, None),
      (r#"1ff"#, Some(0x1ff)),
      (r#"abcd"#, Some(0xabcd)),
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(
        read_hex_n(&mut input.chars().peekable(), input.len()),
        expected
      );
    }
  }

  #[test]
  fn test_read_hex_until_brace() {
    let tests = [
      (r#"{1f}"#, Some(0x1f)),
      (r#"{001f}"#, Some(0x1f)),
      (r#"{1g}"#, None),
      (r#"{001g}"#, None),
      (r#"{1ff}"#, Some(0x1ff)),
      (r#"{abcd}"#, Some(0xabcd)),
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(
        read_hex_until_brace(&mut input.chars().peekable()),
        expected,
      );
    }
  }

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
    assert_lint_err! {
      NoControlRegex,
      r#"/\x1f/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/\u001f/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/\u{001f}/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/\u{0001f}/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/\\\x1f\\x1e/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/\\\x1fFOO\\x00/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"/FOO\\\x1fFOO\\x1f/"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"new RegExp('\\x1f\\x1e')"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"new RegExp('\\x1fFOO\\x00')"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"new RegExp('FOO\\x1fFOO\\x1f')"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r#"RegExp('\\x1f')"#: [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ]
    };
  }
}
