// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  CallExpr, Callee, Expr, Lit, NewExpr, ObjectLit, Prop, PropName, PropOrSpread,
};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoInvalidFetchOptions;

const CODE: &str = "no-invalid-fetch-options";

#[derive(Display)]
enum NoInvalidFetchOptionsMessage {
  #[display(fmt = "\"body\" is not allowed when method is \"{}\"", _0)]
  NotAllowed(String),
}

#[derive(Display)]
enum NoInvalidFetchOptionsHint {
  #[display(
    fmt = "Remove the \"body\", or use a method other than \"GET\" or \"HEAD\""
  )]
  RemoveBodyOrChangeMethod,
}

impl LintRule for NoInvalidFetchOptions {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoInvalidFetchOptionsHandler.traverse(program, context);
  }
}

struct NoInvalidFetchOptionsHandler;

impl Handler for NoInvalidFetchOptionsHandler {
  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    let Callee::Expr(Expr::Ident(ident)) = &call_expr.callee else {
      return;
    };
    if ident.sym() != "fetch" || call_expr.args.len() < 2 {
      return;
    }
    if let Expr::Object(obj) = &call_expr.args[1].expr {
      check_options(obj, ctx);
    }
  }

  fn new_expr(&mut self, new_expr: &NewExpr, ctx: &mut Context) {
    let Expr::Ident(ident) = &new_expr.callee else {
      return;
    };
    if ident.sym() != "Request" {
      return;
    }
    let Some(args) = new_expr.args else {
      return;
    };
    if args.len() < 2 {
      return;
    }
    if let Expr::Object(obj) = &args[1].expr {
      check_options(obj, ctx);
    }
  }
}

fn check_options(obj: &ObjectLit, ctx: &mut Context) {
  // `fetch` and `Request` default the method to "GET".
  let mut method = String::from("GET");
  let mut body_range: Option<SourceRange> = None;

  for prop in obj.props {
    match prop {
      // A spread may introduce a `body` or `method` we cannot statically
      // analyze, so bail out entirely.
      PropOrSpread::Spread(_) => return,
      PropOrSpread::Prop(prop) => match prop {
        Prop::Shorthand(ident) if ident.sym() == "body" => {
          body_range = Some(ident.range());
        }
        // A shorthand `method` references a variable we cannot read
        // statically, so the method is unknown.
        Prop::Shorthand(ident) if ident.sym() == "method" => {
          method = UNKNOWN_METHOD.to_string();
        }
        Prop::KeyValue(kv) => {
          let PropName::Ident(key) = &kv.key else {
            continue;
          };
          if key.sym() == "body" {
            if is_null_or_undefined(&kv.value) {
              body_range = None;
            } else {
              body_range = Some(key.range());
            }
          } else if key.sym() == "method" {
            method = read_method(&kv.value);
          }
        }
        _ => {}
      },
    }
  }

  if method == "GET" || method == "HEAD" {
    if let Some(range) = body_range {
      ctx.add_diagnostic_with_hint(
        range,
        CODE,
        NoInvalidFetchOptionsMessage::NotAllowed(method),
        NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
      );
    }
  }
}

fn is_null_or_undefined(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Ident(ident) => ident.sym() == "undefined",
    _ => false,
  }
}

const UNKNOWN_METHOD: &str = "UNKNOWN";

fn read_method(expr: &Expr) -> String {
  match expr {
    Expr::Lit(Lit::Str(s)) => s.value().to_string_lossy().to_ascii_uppercase(),
    Expr::Tpl(tpl) => {
      if let Some(quasi) = tpl.quasis.first() {
        if quasi.tail() {
          return quasi.raw().as_str().to_ascii_uppercase();
        }
      }
      UNKNOWN_METHOD.to_string()
    }
    _ => UNKNOWN_METHOD.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_invalid_fetch_options.rs
  // MIT Licensed.

  #[test]
  fn no_invalid_fetch_options_valid() {
    assert_lint_ok! {
      NoInvalidFetchOptions,
      r#"fetch(url, {method: "POST", body})"#,
      r#"new Request(url, {method: "POST", body})"#,
      "fetch(url, {})",
      "new Request(url, {})",
      "fetch(url)",
      "new Request(url)",
      r#"fetch(url, {method: "UNKNOWN", body})"#,
      r#"new Request(url, {method: "UNKNOWN", body})"#,
      "fetch(url, {body: undefined})",
      "new Request(url, {body: undefined})",
      "fetch(url, {body: null})",
      "new Request(url, {body: null})",
      "fetch(url, {...options, body})",
      "new Request(url, {...options, body})",
      "new fetch(url, {body})",
      "Request(url, {body})",
      "not_fetch(url, {body})",
      "new not_Request(url, {body})",
      "fetch({body}, url)",
      "new Request({body}, url)",
      r#"fetch(url, {[body]: "foo=bar"})"#,
      r#"new Request(url, {[body]: "foo=bar"})"#,
      r#"fetch(url, {body: "foo=bar", body: undefined});"#,
      r#"new Request(url, {body: "foo=bar", body: undefined});"#,
      r#"fetch(url, {method: "HEAD", body: "foo=bar", method: "post"});"#,
      r#"new Request(url, {method: "HEAD",body: "foo=bar", method: "POST"});"#,
      r#"fetch('/', {body: new URLSearchParams({ data: "test" }), method: "POST"})"#,
      r#"const method = "post"; new Request(url, {method, body: "foo=bar"})"#,
      r#"const method = "post"; fetch(url, {method, body: "foo=bar"})"#,
      r#"const method = `post`; fetch(url, {method, body: "foo=bar"})"#,
      r#"const method = `po${"st"}`; fetch(url, {method, body: "foo=bar"})"#,
      r#"function foo(method: "POST" | "PUT", body: string) {
            return new Request(url, {method, body});
        }"#,
      "function foo(method: string, body: string) {
            return new Request(url, {method, body});
        }",
      r#"enum Method {
          Post = "POST",
        }
        const response = await fetch("/", {
         method: Method.Post,
         body: "",
        });"#,
      "const response = await fetch('', { method, headers, body, });",
      r#"fetch("/url", { method: logic ? "PATCH" : "POST", body: "some body" });"#,
      r#"new Request("/url", { method: logic ? "PATCH" : "POST", body: "some body" });"#,
      r#"fetch("/url", { method: getMethod(), body: "some body" });"#,
      r#"const method = 'POST' as const; await fetch('some-url', { method, body: '' });"#,
      r#"const options = { method: 'POST' } as const; await fetch('some-url', { method: options.method, body: '' });"#,
      r#"const options = { method: 'POST' }; await fetch('some-url', { method: options.method, body: '' });"#,
      r#"const options = { method: 'POST' } as const; new Request('some-url', { method: options.method, body: '' });"#,
      r#"fetch("/url", { method: getOptions().method, body: "some body" });"#,
      r#"new Request("/url", { method: getOptions().method, body: "some body" });"#,
      r#"fetch("/url", { method: (options).method, body: "some body" });"#,
      r#"new Request("/url", { method: (options).method, body: "some body" });"#,
    };
  }

  #[test]
  fn no_invalid_fetch_options_invalid() {
    assert_lint_err! {
      NoInvalidFetchOptions,
      "fetch(url, {body})": [
        {
          col: 12,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      "new Request(url, {body})": [
        {
          col: 18,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {method: "GET", body})"#: [
        {
          col: 27,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"new Request(url, {method: "GET", body})"#: [
        {
          col: 33,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {method: "HEAD", body})"#: [
        {
          col: 28,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"new Request(url, {method: "HEAD", body})"#: [
        {
          col: 34,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {method: "head", body})"#: [
        {
          col: 28,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {method: `head`, body: "foo=bar"})"#: [
        {
          col: 28,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"new Request(url, {method: "head", body})"#: [
        {
          col: 34,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      "fetch(url, {body}, extraArgument)": [
        {
          col: 12,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      "new Request(url, {body}, extraArgument)": [
        {
          col: 18,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {body: undefined, body: "foo=bar"});"#: [
        {
          col: 29,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"new Request(url, {body: undefined, body: "foo=bar"});"#: [
        {
          col: 35,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch(url, {method: "post", body: "foo=bar", method: "HEAD"});"#: [
        {
          col: 28,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"new Request(url, {method: "post", body: "foo=bar", method: "HEAD"});"#: [
        {
          col: 34,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "HEAD"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ],
      r#"fetch('/', {body: new URLSearchParams({ data: "test" })})"#: [
        {
          col: 12,
          message: variant!(NoInvalidFetchOptionsMessage, NotAllowed, "GET"),
          hint: NoInvalidFetchOptionsHint::RemoveBodyOrChangeMethod,
        }
      ]
    };
  }
}
