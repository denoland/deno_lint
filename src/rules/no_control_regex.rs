// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::extract_regex;
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, NewExpr, Regex};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoControlRegex;

const CODE: &str = "no-control-regex";

#[derive(Display)]
enum NoControlRegexMessage {
  #[display(
    fmt = "Unexpected control character literal in regular expression: \\x{:0>2x}.",
    _0
  )]
  Unexpected(u64),
}

#[derive(Display)]
enum NoControlRegexHint {
  #[display(
    fmt = "Disable the rule if the control character literal was intentional, otherwise rework your RegExp"
  )]
  DisableOrRework,
}

impl LintRule for NoControlRegex {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
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

fn add_diagnostic(range: SourceRange, cp: u64, ctx: &mut Context) {
  ctx.add_diagnostic_with_hint(
    range,
    CODE,
    NoControlRegexMessage::Unexpected(cp),
    NoControlRegexHint::DisableOrRework,
  );
}

fn is_control_char(ch: u64) -> bool {
  // This includes all C0 control chars, including \t (0x09), \r (0x0d), and \n (0x0a).
  // It also includes DEL (0x7f) but excludes space (0x20).
  ch <= 0x1f || ch == 0x7f
}

fn check_regex(regex: &str, range: SourceRange, ctx: &mut Context) {
  let mut iter = regex.chars().peekable();
  while let Some(ch) = iter.next() {
    let cp: u64 = ch.into();
    if is_control_char(cp) {
      add_diagnostic(range, cp, ctx);
      return;
    }
  }
}

impl Handler for NoControlRegexHandler {
  fn regex(&mut self, regex: &Regex, ctx: &mut Context) {
    check_regex(regex.inner.exp.to_string().as_str(), regex.range(), ctx);
  }

  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    if let Expr::Ident(ident) = new_expr.callee {
      if let Some(args) = &new_expr.args {
        if let Some(regex) = extract_regex(ctx.scope(), ident, args) {
          check_regex(regex.as_str(), new_expr.range(), ctx);
        }
      }
    }
  }

  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let Callee::Expr(Expr::Ident(ident)) = &call_expr.callee {
      if let Some(regex) = extract_regex(ctx.scope(), ident, call_expr.args) {
        check_regex(regex.as_str(), call_expr.range(), ctx);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_control_regex_valid() {
    assert_lint_ok! {
      NoControlRegex,
      r"/x1f/",
      r"/\\x1f/",
      r"/u001f/",
      r"/\\u001f/",
      r"/u{001f}/",
      r"/\\u{001f}/",
      r"/u{0001f}/",
      r"/\\u{0001f}/",
      r"new RegExp('x1f')",
      r"RegExp('x1f')",
      r"new RegExp('[')",
      r"RegExp('[')",
      r"new (function foo(){})('\\x1f')",
      r"/\x1f/",
      r"/\u001f/",
      r"/\u{001f}/",
      r"/\u{0001f}/",
      r"/\\\x1f\\x1e/",
      r"/\\\x1fFOO\\x00/",
      r"/FOO\\\x1fFOO\\x1f/",
      r"new RegExp('\\x1f\\x1e')",
      r"new RegExp('\\x1fFOO\\x00')",
      r"new RegExp('FOO\\x1fFOO\\x1f')",
      r"RegExp('\\x1f')",
      r"/ /",
      r"RegExp(' ')",
      r"/\t/",
    };
  }

  #[test]
  fn no_control_regex_invalid() {
    assert_lint_err! {
      NoControlRegex,
      "/\u{0000}/": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x00),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      "/\u{001f}/": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      "/\u{007f}/": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x7f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r"new RegExp('\x1f')": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r"new RegExp('\u001f')": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r"new RegExp('\u{1f}')": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      r"new RegExp('\u{0001f}')": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      "new RegExp('x\u{001f}')": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x1f),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
      "/\t/": [
        {
          col: 0,
          message: NoControlRegexMessage::Unexpected(0x09),
          hint: NoControlRegexHint::DisableOrRework,
        }
      ],
    };
  }

  #[test]
  fn no_control_regex_message() {
    assert_eq!(
      NoControlRegexMessage::Unexpected(0x1f).to_string(),
      r"Unexpected control character literal in regular expression: \x1f."
    );

    assert_eq!(
      NoControlRegexMessage::Unexpected(0x00).to_string(),
      r"Unexpected control character literal in regular expression: \x00."
    );
  }
}
