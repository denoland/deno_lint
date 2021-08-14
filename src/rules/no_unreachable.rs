// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_common::Spanned;
use swc_ecmascript::ast::{Decl, Stmt, VarDecl, VarDeclKind};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct NoUnreachable;

const CODE: &str = "no-unreachable";
const MESSAGE: &str = "This statement is unreachable";

impl LintRule for NoUnreachable {
  fn new() -> Box<Self> {
    Box::new(NoUnreachable)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoUnreachableVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_unreachable.md")
  }
}

struct NoUnreachableVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoUnreachableVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoUnreachableVisitor<'c, 'view> {
  fn visit_stmt(&mut self, stmt: &Stmt, _: &dyn Node) {
    stmt.visit_children_with(self);

    match stmt {
      // Don't print unused error for block statements
      Stmt::Block(_) => return,
      // Hoisted, so reachable.
      Stmt::Decl(Decl::Fn(..)) => return,
      // Ignore type declarations.
      Stmt::Decl(Decl::TsInterface(..)) => return,
      Stmt::Decl(Decl::TsTypeAlias(..)) => return,
      Stmt::Decl(Decl::TsModule(..)) => return,
      Stmt::Decl(Decl::Var(VarDecl {
        kind: VarDeclKind::Var,
        decls,
        ..
      }))
        if decls.iter().all(|decl| decl.init.is_none()) =>
      {
        return;
      }
      _ => {}
    }

    if let Some(meta) = self.context.control_flow().meta(stmt.span().lo) {
      if meta.unreachable {
        self.context.add_diagnostic(stmt.span(), CODE, MESSAGE)
      }
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
