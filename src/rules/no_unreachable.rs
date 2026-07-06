// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::tags;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoUnreachable;

const CODE: &str = "no-unreachable";
const MESSAGE: &str = "This statement is unreachable";

impl LintRule for NoUnreachable {
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
    let mut handler = NoUnreachableHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoUnreachableHandler;

fn check_unreachable(span: Span, ctx: &mut Context) {
  if let Some(meta) = ctx.control_flow().meta(span.start) {
    if meta.unreachable {
      ctx.add_diagnostic(span, CODE, MESSAGE);
    }
  }
}

/// Check a statement for unreachability, skipping those that should be ignored
/// (block statements, function declarations, type declarations, and var
/// declarations without initializers).
fn check_statement(stmt: &Statement, ctx: &mut Context) {
  match stmt {
    // Don't print unused error for block statements
    Statement::BlockStatement(_) => {}
    // Hoisted, so reachable.
    Statement::FunctionDeclaration(_) => {}
    // Ignore type declarations.
    Statement::TSInterfaceDeclaration(_) => {}
    Statement::TSTypeAliasDeclaration(_) => {}
    Statement::TSModuleDeclaration(_) => {}
    Statement::VariableDeclaration(decl)
      if decl.kind == VariableDeclarationKind::Var
        && decl.declarations.iter().all(|d| d.init.is_none()) => {}
    _ => {
      check_unreachable(stmt.span(), ctx);
    }
  }
}

impl Handler<'_> for NoUnreachableHandler {
  // We need to check every statement for unreachability.
  // The handler fires for each specific statement type,
  // but we need to intercept at the block/body level to check
  // each statement in sequence.
  //
  // We hook into program, block_statement, switch_case, and other containers
  // that hold statements.
  fn program(&mut self, n: &Program, ctx: &mut Context) {
    for stmt in &n.body {
      check_statement(stmt, ctx);
    }
  }

  fn block_statement(&mut self, n: &BlockStatement, ctx: &mut Context) {
    for stmt in &n.body {
      check_statement(stmt, ctx);
    }
  }

  fn switch_case(&mut self, n: &SwitchCase, ctx: &mut Context) {
    for stmt in &n.consequent {
      check_statement(stmt, ctx);
    }
  }

  fn function(&mut self, n: &Function, ctx: &mut Context) {
    if let Some(body) = &n.body {
      for stmt in &body.statements {
        check_statement(stmt, ctx);
      }
    }
  }

  fn arrow_function_expression(
    &mut self,
    n: &ArrowFunctionExpression,
    ctx: &mut Context,
  ) {
    for stmt in &n.body.statements {
      check_statement(stmt, ctx);
    }
  }

  fn static_block(&mut self, n: &StaticBlock, ctx: &mut Context) {
    for stmt in &n.body {
      check_statement(stmt, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unreachable_valid() {
    assert_lint_ok! {
      NoUnreachable,
      "function foo() {
          function bar() { return 1; }
          return bar();
      }",

      "function foo() {
        return bar();
        function bar() {
          return 1;
        }
      }",

      "function foo() { return x; var x; }",
      "function foo() { var x = 1; var y = 2; }",
      "function foo() { var x = 1; var y = 2; return; }",
      "while (true) { switch (foo) { case 1: x = 1; x = 2;} }",
      "while (true) { break; var x; }",
      "while (true) { continue; var x, y; }",
      "while (true) { throw 'message'; var x; }",
      "while (true) { if (true) break; var x = 1; }",
      "while (true) continue;",
      "switch (foo) { case 1: break; var x; }",

      "switch (foo) {
          case 1:
            break;
            var x;
          default:
            throw true;
        }",

      "const arrow_direction = arrow => {  switch (arrow) { default: throw new Error();  };}",
      "var x = 1; y = 2; throw 'uh oh'; var y;",
      "function foo() { var x = 1; if (x) { return; } x = 2; }",
      "function foo() { var x = 1; if (x) { } else { return; } x = 2; }",

      r#"
function foo() {
  var x = 1;
  switch (x) {
    case 0:
      break;
    default:
      return;
  }
  x = 2;
}
"#,

      "function foo() { var x = 1; while (x) { return; } x = 2; }",
      // https://github.com/denoland/deno_lint/issues/477
      "function foo() { for (;false;) { return 0; } return 1; }",
      "function foo() { var x = 1; for (let i = 0; i < bar(); i++) { return; } x = 2; }",
      r#"
function foo() {
  const partsA = [];
  const partsB = [];
  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const partA = partsA[i];
    const partB = partsB[i];
    if (partA === undefined) return -1;
    if (partB === undefined) return 1;
    if (partA === partB) continue;
    const priorityA = partA.startsWith(":") ? partA.endsWith("*") ? 0 : 1 : 2;
    const priorityB = partB.startsWith(":") ? partB.endsWith("*") ? 0 : 1 : 2;
    return Math.max(Math.min(priorityB - priorityA, 1), -1);
  }
  return 0;
}
      "#,

      "function foo() { var x = 1; for (x in {}) { return; } x = 2; }",
      "function foo() { var x = 1; for (x of []) { return; } x = 2; }",
      "function foo() { var x = 1; try { return; } finally { x = 2; } }",
      "function foo() { var x = 1; for (;;) { if (x) break; } x = 2; }",
      "A: { break A; } foo()",
      "function* foo() { try { yield 1; return; } catch (err) { return err; } }",
      "function foo() { try { bar(); return; } catch (err) { return err; } }",
      "function foo() { try { a.b.c = 1; return; } catch (err) { return err; } }",
      "function foo() { try { a.b.c = 1; } catch (err) { c.b.a = 1; } finally { return; } }",
      "function foo() { try { a.b.c = 1; } catch (err) { return; } finally { return; } }",
      "function foo() { try { bar(); return; } catch (err) { a.b = 1; } finally { return; } }",
      "function foo() { try { bar(); return; } catch (err) { return; } finally { return; } }",

      r#"
function normalize(type: string): string | undefined {
  switch (type) {
    case "urlencoded":
      return "application/x-www-form-urlencoded";
    case "multipart":
      return "multipart/*";
  }
  if (type[0] === "+") {
    return `*/*${type}`;
  }
  return type.includes("/") ? type : lookup(type);
}
"#,

      "A: {
        switch (5) {
          case 1:
            return 'foo';
          case 5:
            break A;
        }
      }
      call();
      ",

      "A: {
        switch (5) {
          case 1:
            break
          case 5:
            break A;
        }
      }
      call();
      ",

      r#"
      switch (vers) {
        case "HTTP/1.1":
          return [1, 1];

        case "HTTP/1.0":
          return [1, 0];

        default: {
          const Big = 1000000; // arbitrary upper bound

          if (!vers.startsWith("HTTP/")) {
            break;
          }

          const dot = vers.indexOf(".");
          if (dot < 0) {
            break;
          }

          const majorStr = vers.substring(vers.indexOf("/") + 1, dot);
          const major = Number(majorStr);
          if (!Number.isInteger(major) || major < 0 || major > Big) {
            break;
          }

          const minorStr = vers.substring(dot + 1);
          const minor = Number(minorStr);
          if (!Number.isInteger(minor) || minor < 0 || minor > Big) {
            break;
          }

          return [major, minor];
        }
      }

      throw new Error(`malformed HTTP version ${vers}`);"#,

      // https://github.com/denoland/deno_lint/issues/340
      r#"
      function foo() {
        let ret = "";
        let p: BufferListItem | null = (this.head as BufferListItem);
        let c = 0;
        p = p.next as BufferListItem;
        do {
          const str = p.data;
          if (n > str.length) {
            ret += str;
            n -= str.length;
          } else {
            if (n === str.length) {
              ret += str;
              ++c;
              if (p.next) {
                this.head = p.next;
              } else {
                this.head = this.tail = null;
              }
            } else {
              ret += str.slice(0, n);
              this.head = p;
              p.data = str.slice(n);
            }
            break;
          }
          ++c;
          p = p.next;
        } while (p);
        this.length -= c;
        return ret;
      }
      "#,

      r#"
      function foo() {
        let ret = "";
        do {
          const str = p.data;
          if (n > str.length) {
            ret += str;
          } else {
            if (n === str.length) {
              ret += str;
              if (p.next) {
                this.head = p.next;
              } else {
                this.head = this.tail = null;
              }
            } else {
              p.data = str.slice(n);
            }
            break;
          }
          p = p.next;
        } while (p);
        return ret;
      }
      "#,

      r#"
      function foo() {
        let ret = "";
          while(p) {
            const str = p.data;
            if (n > str.length) {
              ret += str;
            } else {
              if (n === str.length) {
                ret += str;
                if (p.next) {
                  this.head = p.next;
                } else {
                  this.head = this.tail = null;
                }
              } else {
                p.data = str.slice(n);
              }
              break;
            }
          p = p.next;
        }
        return ret;
      }
      "#,

      // https://github.com/denoland/deno_lint/issues/353
      r#"
class Class {
  constructor() {
    return this;
  }
}

console.log("unreachable???");
      "#,

      r#"
class Class {
  constructor() {
    if (Deno) return this;
  }
}

console.log("unreachable???");
      "#,

      // https://github.com/denoland/deno_lint/issues/582
      r#"
try {
  try {
    throw new Error();
  } catch {
    throw new Error();
  }
} catch {
  console.log("Statement reached!");
}
      "#,
      r#"
try {
  try {
    throw new Error();
  } catch {
    foo();
  }
} catch {
  console.log("Statement reached!");
}
      "#,
      r#"
throw 'oops';
interface I<V> {
  k: V;
}
      "#,
      r#"
throw new Error();
type S = string;
type X<T> = T;
      "#,
      r#"
throw new Error();
declare module "SomeModule" {
  export function fn(): void;
}
      "#,

      // https://github.com/denoland/deno_lint/issues/674
      r#"
const a = "foo";
while (true) {
  if (a == "foo") {
    break;
  }
  throw new Error("bar");
}
console.log("foobar");
      "#,
      r#"
const a = "foo";
do {
  if (a == "foo") {
    break;
  }
  throw new Error("bar");
} while (true);
console.log("foobar");
      "#,

      // https://github.com/denoland/deno_lint/issues/716
      r#"
function foo(trueOrFalse: boolean) {
  if (trueOrFalse) {
    // noop
  } else {
    // noop
  }
  try {
    bar();
    return 42;
  } catch (err) {
    console.error(err);
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/1468
      r#"
function test(): boolean {
  let tryCount = 0;
  do {
    const httpCode = genRandomNumber(200, 700);
    if (isRetryableError(httpCode)) {
      continue;
    }
    return randomlyErrorOut();
  } while (tryCount++ < retryCount);
  throw new Error("Exceeded maximum retry attempts");
}
      "#,

      // https://github.com/denoland/deno_lint/issues/1341
      r#"
let one: number | undefined;
mainLoop: for (;;) {
  let two: number | undefined;
  for (;;) {
    if (one === 10) {
      break mainLoop;
    }
    if (two === 10) {
      break;
    }
    two = (two ?? 0) + 1;
  }
  one = (one ?? 0) + 1;
  console.log(one);
}
console.log("finished!");
      "#,

      // https://github.com/denoland/deno_lint/issues/1303
      r#"
function f() {
  const fooError = new Error("foo");
  try {
    throw fooError;
  } catch (cause) {
    assert(cause instanceof Error && cause.message === "foo");
    throw new Error("bar", { cause });
  }
}
      "#,

      // `Deno.exit()` / `process.exit()` in *expression* position must not mark
      // the following statements as unreachable.
      r#"
function h() {
  const port = Deno.env.get("PORT") ?? Deno.exit(1);
  startServer(port);
}
      "#,
      "function g(cond) { const result = cond || Deno.exit(1); return result; }",
      "function p(cond) { const x = cond ? 1 : process.exit(); console.log(x); }",
    };
  }

  #[test]
  fn no_unreachable_invalid() {
    assert_lint_err! {
        NoUnreachable,
        "function foo() { return x; var x = 1; }": [{ col: 27, message: MESSAGE }],
        "function foo() { return x; var x, y = 1; }": [{ col: 27, message: MESSAGE }],
        "while (true) { continue; var x = 1; }": [{ col: 25, message: MESSAGE }],
        "function foo() { return; x = 1; }": [{ col: 25, message: MESSAGE }],
        "function foo() { throw error; x = 1; }": [{ col: 30, message: MESSAGE }],
        "while (true) { break; x = 1; }": [{ col: 22, message: MESSAGE }],
        "while (true) { continue; x = 1; }": [{ col: 25, message: MESSAGE }],
        "function foo() { switch (foo) { case 1: return; x = 1; } }": [{ col: 48, message: MESSAGE }],
        "function foo() { switch (foo) { case 1: throw e; x = 1; } }": [{ col: 49, message: MESSAGE }],
        "while (true) { switch (foo) { case 1: break; x = 1; } }": [{ col: 45, message: MESSAGE }],
        "while (true) { switch (foo) { case 1: continue; x = 1; } }": [{ col: 48, message: MESSAGE }],
        "var x = 1; throw 'uh oh'; var y = 2;": [{ col: 26, message: MESSAGE }],
        "function foo() { var x = 1; if (x) { return; } else { throw e; } x = 2; }": [{ col: 65, message: MESSAGE }],
        "function foo() { var x = 1; if (x) return; else throw -1; x = 2; }": [{ col: 58, message: MESSAGE }],
        "function foo() { var x = 1; try { return; } finally {} x = 2; }": [{ col: 55, message: MESSAGE }],
        "function foo() { var x = 1; try { } finally { return; } x = 2; }": [{ col: 56, message: MESSAGE }],
        "function foo() { var x = 1; do { return; } while (x); x = 2; }": [{ col: 54, message: MESSAGE }],
        "function foo() { var x = 1; while (x) { if (x) break; else continue; x = 2; } }": [{ col: 69, message: MESSAGE }],
        "function foo() { var x = 1; for (;;) { if (x) continue; } x = 2; }": [{ col: 58, message: MESSAGE }],
        "function foo() { var x = 1; for (;true;) { if (x) continue; } x = 2; }": [{ col: 62, message: MESSAGE }],
        "function foo() { var x = 1; while (true) { } x = 2; }": [{ col: 45, message: MESSAGE }],
        "const arrow_direction = arrow => {
        switch (arrow) {
          default:
            throw new Error();
        }
        g()
      }": [{ line: 6, col: 8, message: MESSAGE }],
        "function foo() {
      return;
      a();
      b()
      // comment
      c();
  }": [{ line: 3, col: 6, message: MESSAGE }, {line: 4, col: 6, message: MESSAGE }, { line: 6, col: 6, message: MESSAGE }],
        "function foo() {
      if (a) {
          return
          b();
          c();
      } else {
          throw err
          d();
      }
  }": [{ line: 4, col: 10, message: MESSAGE }, { line: 5, col: 10, message: MESSAGE }, { line: 8, col: 10, message: MESSAGE }],
        "function foo() {
      if (a) {
          return
          b();
          c();
      } else {
          throw err
          d();
      }
      e();
  }": [{ line: 4, col: 10, message: MESSAGE }, { line: 5, col: 10, message: MESSAGE }, { line: 8, col: 10, message: MESSAGE}, { line: 10, col: 6, message: MESSAGE }],
        "function* foo() {
      try {
          return;
      } catch (err) {
          return err;
      }
  }": [{ line: 5, col: 10, message: MESSAGE }],
        "function foo() {
      try {
          return;
      } catch (err) {
          return err;
      }
  }": [{ line: 5, col: 10, message: MESSAGE }],
        "function foo() {
      try {
          return;
          let a = 1;
      } catch (err) {
          return err;
      }
  }": [{ line: 4, col: 10, message: MESSAGE }, { line: 6, col: 10, message: MESSAGE }],
      // https://github.com/denoland/deno_lint/issues/348
        r#"
const obj = {
  get root() {
    let primary = this;
    while (true) {
      if (primary.parent !== undefined) {
          primary = primary.parent;
      } else {
          return primary;
      }
    }
    return 1;
  }
};
      "#: [{ line: 12, col: 4, message: MESSAGE }],
    }
  }
}
