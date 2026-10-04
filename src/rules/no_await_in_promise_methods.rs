// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Callee, Expr, MemberProp};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoAwaitInPromiseMethods;

const CODE: &str = "no-await-in-promise-methods";

#[derive(Display)]
enum NoAwaitInPromiseMethodsMessage {
  #[display(fmt = "Promise in `Promise.{}()` should not be awaited.", _0)]
  Unexpected(String),
}

#[derive(Display)]
enum NoAwaitInPromiseMethodsHint {
  #[display(fmt = "Remove the `await`")]
  Remove,
}

impl LintRule for NoAwaitInPromiseMethods {
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
    NoAwaitInPromiseMethodsHandler.traverse(program, context);
  }
}

const PROMISE_METHODS: [&str; 4] = ["all", "allSettled", "any", "race"];

struct NoAwaitInPromiseMethodsHandler;

impl Handler for NoAwaitInPromiseMethodsHandler {
  fn call_expr(
    &mut self,
    call_expr: &deno_ast::view::CallExpr,
    context: &mut Context,
  ) {
    let Callee::Expr(Expr::Member(member_expr)) = &call_expr.callee else {
      return;
    };

    // Object must be the `Promise` identifier.
    let Expr::Ident(obj) = &member_expr.obj else {
      return;
    };
    if obj.sym() != "Promise" {
      return;
    }

    // Property must be a non-computed identifier among the matched methods.
    let MemberProp::Ident(prop) = &member_expr.prop else {
      return;
    };
    let method_name = prop.sym();
    if !PROMISE_METHODS.contains(&method_name.as_ref()) {
      return;
    }

    // The sole argument must be an array literal (not a spread).
    if call_expr.args.len() != 1 {
      return;
    }
    let arg = call_expr.args[0];
    if arg.inner.spread.is_some() {
      return;
    }
    let Expr::Array(array_lit) = &arg.expr else {
      return;
    };

    for elem in array_lit.elems.iter().flatten() {
      if elem.inner.spread.is_some() {
        continue;
      }
      if let Expr::Await(await_expr) = &elem.expr {
        context.add_diagnostic_with_hint(
          await_expr.range(),
          CODE,
          NoAwaitInPromiseMethodsMessage::Unexpected(method_name.to_string()),
          NoAwaitInPromiseMethodsHint::Remove,
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_await_in_promise_methods.rs
  // MIT Licensed.

  #[test]
  fn no_await_in_promise_methods_valid() {
    assert_lint_ok! {
      NoAwaitInPromiseMethods,
      "Promise.all([promise1, promise2, promise3, promise4])",
      "Promise.allSettled([promise1, promise2, promise3, promise4])",
      "Promise.any([promise1, promise2, promise3, promise4])",
      "Promise.race([promise1, promise2, promise3, promise4])",
      "Promise.all(...[await promise])",
      "Promise.all([await promise], extraArguments)",
      "Promise.all()",
      "Promise.all(notArrayExpression)",
      "Promise.all([,])",
      "Promise[all]([await promise])",
      "Promise.all?.([await promise])",
      "Promise?.all([await promise])",
      "Promise.notListedMethod([await promise])",
      "NotPromise.all([await promise])",
      "Promise.all([(await promise, 0)])",
      "new Promise.all([await promise])",
      "globalThis.Promise.all([await promise])",
      r#"Promise["all"]([await promise])"#,
    };
  }

  #[test]
  fn no_await_in_promise_methods_invalid() {
    assert_lint_err! {
      NoAwaitInPromiseMethods,
      "Promise.all([await promise])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.allSettled([await promise])": [
        {
          col: 20,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "allSettled"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.any([await promise])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "any"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.race([await promise])": [
        {
          col: 14,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "race"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([, await promise])": [
        {
          col: 15,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await promise,])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await promise],)": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await (0, promise)],)": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await (( promise ))])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await await promise])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([...foo, await promise1, await promise2])": [
        {
          col: 21,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        },
        {
          col: 37,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ],
      "Promise.all([await /* comment*/ promise])": [
        {
          col: 13,
          message: variant!(NoAwaitInPromiseMethodsMessage, Unexpected, "all"),
          hint: NoAwaitInPromiseMethodsHint::Remove,
        }
      ]
    };
  }
}
