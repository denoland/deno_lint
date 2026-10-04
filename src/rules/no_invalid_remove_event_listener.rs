// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  CallExpr, Callee, Expr, ExprOrSpread, MemberExpr, MemberProp, OptCall,
  OptChainBase,
};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoInvalidRemoveEventListener;

const CODE: &str = "no-invalid-remove-event-listener";

#[derive(Display)]
enum NoInvalidRemoveEventListenerMessage {
  #[display(fmt = "Invalid `removeEventListener` call.")]
  Invalid,
}

#[derive(Display)]
enum NoInvalidRemoveEventListenerHint {
  #[display(fmt = "The listener argument should be a function reference.")]
  FunctionReference,
}

impl LintRule for NoInvalidRemoveEventListener {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoInvalidRemoveEventListenerHandler.traverse(program, context);
  }
}

/// Unwraps a callee expression into a (non-computed) member expression,
/// looking through an optional-chaining member access.
fn member_from_expr<'a>(expr: &Expr<'a>) -> Option<&'a MemberExpr<'a>> {
  match expr {
    Expr::Member(member) => Some(*member),
    Expr::OptChain(opt_chain) => match &opt_chain.base {
      OptChainBase::Member(member) => Some(*member),
      OptChainBase::Call(_) => None,
    },
    _ => None,
  }
}

/// Returns `true` if the given call expression is an inline `.bind(...)` call,
/// e.g. `handler.bind(this)`.
fn is_bind_call(call: &CallExpr) -> bool {
  let Callee::Expr(callee) = &call.callee else {
    return false;
  };
  let Some(member) = member_from_expr(callee) else {
    return false;
  };
  matches!(&member.prop, MemberProp::Ident(name) if name.sym().as_str() == "bind")
}

fn check_remove_event_listener(
  callee: &Expr,
  args: &[&ExprOrSpread],
  context: &mut Context,
) {
  let Some(member) = member_from_expr(callee) else {
    return;
  };

  // The method name must be exactly `removeEventListener` and accessed
  // statically (not computed, e.g. `el[removeEventListener]`).
  let MemberProp::Ident(name) = &member.prop else {
    return;
  };
  if name.sym().as_str() != "removeEventListener" {
    return;
  }

  // A spread as the first argument means we can't reliably tell which
  // argument is the listener.
  if matches!(args.first(), Some(arg) if arg.spread().is_some()) {
    return;
  }

  let Some(listener) = args.get(1) else {
    return;
  };

  let is_invalid_listener = match &listener.expr {
    Expr::Arrow(_) | Expr::Fn(_) => true,
    Expr::Call(call) => is_bind_call(call),
    _ => false,
  };

  if is_invalid_listener {
    context.add_diagnostic_with_hint(
      listener.range(),
      CODE,
      NoInvalidRemoveEventListenerMessage::Invalid,
      NoInvalidRemoveEventListenerHint::FunctionReference,
    );
  }
}

struct NoInvalidRemoveEventListenerHandler;

impl Handler for NoInvalidRemoveEventListenerHandler {
  fn call_expr(&mut self, call: &CallExpr, context: &mut Context) {
    if let Callee::Expr(callee) = &call.callee {
      check_remove_event_listener(callee, call.args, context);
    }
  }

  fn opt_call(&mut self, opt_call: &OptCall, context: &mut Context) {
    // An optional *call* (`el.removeEventListener?.(...)`) can't be the form
    // we flag, so skip it.
    if opt_call.parent().optional() {
      return;
    }
    check_remove_event_listener(&opt_call.callee, opt_call.args, context);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_invalid_remove_event_listener.rs
  // MIT Licensed.

  #[test]
  fn no_invalid_remove_event_listener_valid() {
    assert_lint_ok! {
      NoInvalidRemoveEventListener,
      r#"new el.removeEventListener("click", () => {})"#,
      r#"el.removeEventListener?.("click", () => {})"#,
      r#"el.notRemoveEventListener("click", () => {})"#,
      r#"el[removeEventListener]("click", () => {})"#,
      r#"el.removeEventListener("click")"#,
      "el.removeEventListener()",
      "el.removeEventListener(() => {})",
      r#"el.removeEventListener(...["click", () => {}], () => {})"#,
      r#"el.removeEventListener(() => {}, "click")"#,
      r#"window.removeEventListener("click", bind())"#,
      r#"window.removeEventListener("click", handler.notBind())"#,
      r#"window.removeEventListener("click", handler[bind]())"#,
      r#"window.removeEventListener("click", handler.bind?.())"#,
      r#"window.removeEventListener("click", handler?.bind())"#,
      "window.removeEventListener(handler)",
      r#"this.removeEventListener("click", getListener())"#,
      r#"el.removeEventListener("scroll", handler)"#,
      r#"el.removeEventListener("keydown", obj.listener)"#,
      r#"removeEventListener("keyup", () => {})"#,
      r#"removeEventListener("keydown", function () {})"#,
      "document.removeEventListener('keydown', keydownHandler)",
      "document.removeEventListener('keydown', this.keydownHandler)",
    };
  }

  #[test]
  fn no_invalid_remove_event_listener_invalid() {
    assert_lint_err! {
      NoInvalidRemoveEventListener,
      r#"window.removeEventListener("scroll", handler.bind(abc))"#: [
        {
          col: 37,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"window.removeEventListener("scroll", this.handler.bind(abc))"#: [
        {
          col: 37,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"window.removeEventListener("click", () => {})"#: [
        {
          col: 36,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"window.removeEventListener("keydown", function () {})"#: [
        {
          col: 38,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el.removeEventListener("click", (e) => { e.preventDefault(); })"#: [
        {
          col: 32,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el.removeEventListener("mouseover", fn.bind(abc))"#: [
        {
          col: 36,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el?.removeEventListener("mouseover", fn.bind(abc))"#: [
        {
          col: 37,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el.removeEventListener("mouseout", function (e) {})"#: [
        {
          col: 35,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el?.removeEventListener("mouseout", function (e) {})"#: [
        {
          col: 36,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el.removeEventListener("mouseout", function (e) {}, true)"#: [
        {
          col: 35,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      r#"el.removeEventListener("click", function (e) {}, ...moreArguments)"#: [
        {
          col: 32,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      "el.removeEventListener(() => {}, () => {}, () => {})": [
        {
          col: 33,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      "document.removeEventListener('keydown', () => foo())": [
        {
          col: 40,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      "document.removeEventListener('keydown', function () {})": [
        {
          col: 40,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      // Multi-line arrow function: span is shortened in oxc, but the start
      // (line/col) is unchanged.
      r#"
        element.removeEventListener("glider-refresh", event => {
            // $ExpectType GliderEvent<undefined>
            event;

            // $ExpectType boolean
            event.bubbles;

            event.target;

            if (event.target) {
                // $ExpectType Glider<HTMLElement> | undefined
                event.target._glider;
            }
        });
        "#: [
        {
          line: 2,
          col: 54,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ],
      // Multi-line function expression.
      r#"
        element.removeEventListener("glider-refresh", function (event) {
            // $ExpectType GliderEvent<undefined>
            event;

            // $ExpectType boolean
            event.bubbles;

            event.target;

            if (event.target) {
                // $ExpectType Glider<HTMLElement> | undefined
                event.target._glider;
            }
        });
        "#: [
        {
          line: 2,
          col: 54,
          message: NoInvalidRemoveEventListenerMessage::Invalid,
          hint: NoInvalidRemoveEventListenerHint::FunctionReference,
        }
      ]
    };
  }
}
