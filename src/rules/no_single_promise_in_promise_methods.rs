// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Callee, Expr, ExprOrSpread, MemberProp};
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoSinglePromiseInPromiseMethods;

const CODE: &str = "no-single-promise-in-promise-methods";

#[derive(Display)]
enum NoSinglePromiseInPromiseMethodsMessage {
  #[display(
    fmt = "Wrapping single-element array with `Promise.{}()` is unnecessary.",
    _0
  )]
  Unnecessary(String),
}

#[derive(Display)]
enum NoSinglePromiseInPromiseMethodsHint {
  #[display(
    fmt = "Either use the value directly, or switch to `Promise.resolve(…)`."
  )]
  UseDirectly,
}

impl LintRule for NoSinglePromiseInPromiseMethods {
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
    NoSinglePromiseInPromiseMethodsHandler.traverse(program, context);
  }
}

/// Unwraps parenthesized and TypeScript type-modifying expressions (`as`,
/// `satisfies`, non-null assertions, type assertions, `as const`, and
/// instantiation expressions) to reach the underlying expression.
fn unwrap_expr<'a>(expr: &Expr<'a>) -> Expr<'a> {
  match expr {
    Expr::Paren(e) => unwrap_expr(&e.expr),
    Expr::TsAs(e) => unwrap_expr(&e.expr),
    Expr::TsConstAssertion(e) => unwrap_expr(&e.expr),
    Expr::TsSatisfies(e) => unwrap_expr(&e.expr),
    Expr::TsNonNull(e) => unwrap_expr(&e.expr),
    Expr::TsTypeAssertion(e) => unwrap_expr(&e.expr),
    Expr::TsInstantiation(e) => unwrap_expr(&e.expr),
    _ => *expr,
  }
}

struct NoSinglePromiseInPromiseMethodsHandler;

impl Handler for NoSinglePromiseInPromiseMethodsHandler {
  fn call_expr(
    &mut self,
    call_expr: &deno_ast::view::CallExpr,
    ctx: &mut Context,
  ) {
    // The callee must be a static member expression `Promise.<method>`. Note
    // that optional chaining (`Promise?.race(...)` / `Promise.race?.(...)`) is
    // represented as an `OptChainExpr`, so it never reaches this handler.
    let Callee::Expr(Expr::Member(member)) = &call_expr.callee else {
      return;
    };

    // The object must be the identifier `Promise`.
    let Expr::Ident(obj) = &member.obj else {
      return;
    };
    if obj.sym() != "Promise" {
      return;
    }

    // The property must be a non-computed identifier and one of the supported
    // methods.
    let MemberProp::Ident(prop) = &member.prop else {
      return;
    };
    let method_name = prop.sym().as_str();
    if !matches!(method_name, "all" | "any" | "race") {
      return;
    }

    // The sole argument must be an array literal (not a spread argument).
    if call_expr.args.len() != 1 {
      return;
    }
    let arg: &ExprOrSpread = call_expr.args[0];
    if arg.spread().is_some() {
      return;
    }

    let Expr::Array(array) = unwrap_expr(&arg.expr) else {
      return;
    };

    // The array literal must have exactly one element, and that element must be
    // a plain expression (not an elision or a spread element).
    if array.elems.len() != 1 {
      return;
    }
    let Some(elem) = array.elems[0] else {
      return;
    };
    if elem.spread().is_some() {
      return;
    }

    ctx.add_diagnostic_with_hint(
      call_expr.range(),
      CODE,
      NoSinglePromiseInPromiseMethodsMessage::Unnecessary(
        method_name.to_string(),
      ),
      NoSinglePromiseInPromiseMethodsHint::UseDirectly,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_single_promise_in_promise_methods.rs
  // MIT Licensed.

  #[test]
  fn no_single_promise_in_promise_methods_valid() {
    assert_lint_ok! {
      NoSinglePromiseInPromiseMethods,
      "Promise.race([promise, anotherPromise])",
      "Promise.race(notArrayLiteral)",
      "Promise.race([...promises])",
      "Promise.any([promise, anotherPromise])",
      "Promise.notListedMethod([promise])",
      "Promise[race]([promise])",
      "Promise.race([,])",
      "NotPromise.race([promise])",
      "Promise?.race([promise])",
      "Promise.race?.([promise])",
      "Promise.race(...[promise])",
      "Promise.race([promise], extraArguments)",
      "Promise.race()",
      "new Promise.race([promise])",
      // We are not checking these cases
      "globalThis.Promise.race([promise])",
      r#"Promise["race"]([promise])"#,
      // This can't be checked
      "Promise.allSettled([promise])",
    };
  }

  #[test]
  fn no_single_promise_in_promise_methods_invalid() {
    use NoSinglePromiseInPromiseMethodsHint::UseDirectly;

    fn message(method: &str) -> String {
      NoSinglePromiseInPromiseMethodsMessage::Unnecessary(method.to_string())
        .to_string()
    }

    assert_lint_err! {
      NoSinglePromiseInPromiseMethods,
      "await Promise.race([(0, promise)])": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "async function * foo() {await Promise.race([yield promise])}": [
        { col: 30, message: message("race"), hint: UseDirectly }
      ],
      "async function * foo() {await Promise.race([yield* promise])}": [
        { col: 30, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([() => promise,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([a ? b : c,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x ??= y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x ||= y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x &&= y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x |= y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x ^= y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x | y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x ^ y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x & y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x !== y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x == y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x in y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x >>> y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x + y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x / y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([x ** y,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([promise,],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([getPromise(),],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([promises[0],],)": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([await promise])": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.any([promise])": [
        { col: 6, message: message("any"), hint: UseDirectly }
      ],
      "await Promise.race([promise])": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "await Promise.race([new Promise(() => {})])": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      "+await Promise.race([+1])": [
        { col: 7, message: message("race"), hint: UseDirectly }
      ],
      // ASI, `Promise.race()` is not really `await`ed
      "await Promise.race([(x,y)])\n[0].toString()": [
        { col: 6, message: message("race"), hint: UseDirectly }
      ],
      // Not `await`ed
      "Promise.race([promise,],)": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "foo\nPromise.race([(0, promise),],)": [
        { line: 2, col: 0, message: message("race"), hint: UseDirectly }
      ],
      "foo\nPromise.race([[array][0],],)": [
        { line: 2, col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([promise]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([1]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([1.]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([.1]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([(0, promise)]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "const _ = () => Promise.race([ a ?? b ,],)": [
        { col: 16, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([ {a} = 1 ,],)": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([ function () {} ,],)": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([ class {} ,],)": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([ new Foo ,],).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([ new Foo ,],).toString": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "foo(Promise.race([promise]))": [
        { col: 4, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([promise]).foo = 1": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([promise])[0] ||= 1": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([undefined]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      "Promise.race([null]).then()": [
        { col: 0, message: message("race"), hint: UseDirectly }
      ],
      // `Promise.all` specific
      "Promise.all([promise])": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "await Promise.all([promise])": [
        { col: 6, message: message("all"), hint: UseDirectly }
      ],
      "const foo = () => Promise.all([promise])": [
        { col: 18, message: message("all"), hint: UseDirectly }
      ],
      "const foo = await Promise.all([promise])": [
        { col: 18, message: message("all"), hint: UseDirectly }
      ],
      "foo = await Promise.all([promise])": [
        { col: 12, message: message("all"), hint: UseDirectly }
      ],
      // `Promise.{all, race}()` should not care if the result is used
      "const foo = await Promise.race([promise])": [
        { col: 18, message: message("race"), hint: UseDirectly }
      ],
      "const foo = () => Promise.race([promise])": [
        { col: 18, message: message("race"), hint: UseDirectly }
      ],
      "foo = await Promise.race([promise])": [
        { col: 12, message: message("race"), hint: UseDirectly }
      ],
      "const results = await Promise.any([promise])": [
        { col: 22, message: message("any"), hint: UseDirectly }
      ],
      "const results = await Promise.race([promise])": [
        { col: 22, message: message("race"), hint: UseDirectly }
      ],
      "const [foo] = await Promise.all([promise])": [
        { col: 20, message: message("all"), hint: UseDirectly }
      ],
      // TypeScript-specific
      "Promise.all([x] as const).then()": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "Promise.all([x] satisfies any[]).then()": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "Promise.all([x as const]).then()": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "Promise.all([x!]).then()": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "Promise.all(['one']).then(something);": [
        { col: 0, message: message("all"), hint: UseDirectly }
      ],
      "async function run() {\n  await Promise.all([\n    new Promise((resolve) => resolve(true)),\n  ]);\n}": [
        { line: 2, col: 8, message: message("all"), hint: UseDirectly }
      ]
    };
  }
}
