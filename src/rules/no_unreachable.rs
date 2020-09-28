// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Spanned;
use swc_ecmascript::ast::{Decl, Stmt, VarDecl, VarDeclKind};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

use std::sync::Arc;

pub struct NoUnreachable;

impl LintRule for NoUnreachable {
  fn new() -> Box<Self> {
    Box::new(NoUnreachable)
  }

  fn code(&self) -> &'static str {
    "no-unreachable"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoUnreachableVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoUnreachableVisitor {
  context: Arc<Context>,
}

impl NoUnreachableVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoUnreachableVisitor {
  fn visit_stmt(&mut self, stmt: &Stmt, _: &dyn Node) {
    stmt.visit_children_with(self);

    match stmt {
      // Don't print unused error for block statements
      Stmt::Block(_) => return,
      // Hoisted, so reachable.
      Stmt::Decl(Decl::Fn(..)) => return,
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

    if let Some(meta) = self.context.control_flow.meta(stmt.span().lo) {
      if meta.unreachable {
        self.context.add_diagnostic(
          stmt.span(),
          "no-unreachable",
          "This statement is unreachable",
        )
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() {
          function bar() { return 1; }
          return bar();
      }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() {
        return bar();
        function bar() {
          return 1;
        }
      }",
    );

    assert_lint_ok::<NoUnreachable>("function foo() { return x; var x; }");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoUnreachable>("function foo() { var x = 1; var y = 2; }");

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; var y = 2; return; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: x = 1; x = 2;} }",
    );
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoUnreachable>("while (true) { break; var x; }");

    assert_lint_ok::<NoUnreachable>("while (true) { continue; var x, y; }");

    assert_lint_ok::<NoUnreachable>("while (true) { throw 'message'; var x; }");
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoUnreachable>(
      "while (true) { if (true) break; var x = 1; }",
    );

    assert_lint_ok::<NoUnreachable>("while (true) continue;");

    assert_lint_ok::<NoUnreachable>("switch (foo) { case 1: break; var x; }");
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoUnreachable>(
      "switch (foo) {
          case 1:
            break;
            var x;
          default:
            throw true;
        }",
    );

    assert_lint_ok::<NoUnreachable>("const arrow_direction = arrow => {  switch (arrow) { default: throw new Error();  };}");

    assert_lint_ok::<NoUnreachable>("var x = 1; y = 2; throw 'uh oh'; var y;");
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; if (x) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; if (x) { } else { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() {
      var x = 1;
      switch (x) {
        case 0:
          break;
        default:
          return;
      }
      x = 2; }",
    );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; while (x) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; for (x in {}) { return; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; try { return; } finally { x = 2; } }",
    );
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { var x = 1; for (;;) { if (x) break; } x = 2; }",
    );

    assert_lint_ok::<NoUnreachable>("A: { break A; } foo()");

    assert_lint_ok::<NoUnreachable>("function* foo() { try { yield 1; return; } catch (err) { return err; } }");
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoUnreachable>(
      "function foo() { try { bar(); return; } catch (err) { return err; } }",
    );

    assert_lint_ok::<NoUnreachable>("function foo() { try { a.b.c = 1; return; } catch (err) { return err; } }");
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoUnreachable>(
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
    );
  }

  #[test]
  fn ok_break_labeled() {
    assert_lint_ok::<NoUnreachable>(
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
    );

    assert_lint_ok::<NoUnreachable>(
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
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { return x; var x = 1; }",
      27,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { return x; var x, y = 1; }",
      27,
    );

    assert_lint_err::<NoUnreachable>(
      "while (true) { continue; var x = 1; }",
      25,
    );
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoUnreachable>("function foo() { return; x = 1; }", 25);

    assert_lint_err::<NoUnreachable>(
      "function foo() { throw error; x = 1; }",
      30,
    );

    assert_lint_err::<NoUnreachable>("while (true) { break; x = 1; }", 22);
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoUnreachable>("while (true) { continue; x = 1; }", 25);

    assert_lint_err::<NoUnreachable>(
      "function foo() { switch (foo) { case 1: return; x = 1; } }",
      48,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { switch (foo) { case 1: throw e; x = 1; } }",
      49,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: break; x = 1; } }",
      45,
    );

    assert_lint_err::<NoUnreachable>(
      "while (true) { switch (foo) { case 1: continue; x = 1; } }",
      48,
    );

    assert_lint_err::<NoUnreachable>(
      "var x = 1; throw 'uh oh'; var y = 2;",
      26,
    );
  }

  #[test]
  fn err_5() {
    assert_lint_err::<NoUnreachable>("function foo() { var x = 1; if (x) { return; } else { throw e; } x = 2; }", 65);

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; if (x) return; else throw -1; x = 2; }",
      58,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; try { return; } finally {} x = 2; }",
      55,
    );
  }

  #[test]
  fn err_6() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; try { } finally { return; } x = 2; }",
      56,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; do { return; } while (x); x = 2; }",
      54,
    );

    assert_lint_err::<NoUnreachable>("function foo() { var x = 1; while (x) { if (x) break; else continue; x = 2; } }", 69);
  }

  #[test]
  fn err_7() {
    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; for (;;) { if (x) continue; } x = 2; }",
      58,
    );

    assert_lint_err::<NoUnreachable>(
      "function foo() { var x = 1; while (true) { } x = 2; }",
      45,
    );

    assert_lint_err_on_line::<NoUnreachable>(
      "const arrow_direction = arrow => {
        switch (arrow) {
          default:
            throw new Error();
        }
        g()
      }",
      6,
      8,
    );
  }

  #[test]
  fn err_8() {
    assert_lint_err_on_line_n::<NoUnreachable>(
      "function foo() {
      return;
      a();
      b()
      // comment
      c();
  }",
      vec![(3, 6), (4, 6), (6, 6)],
    );

    assert_lint_err_on_line_n::<NoUnreachable>(
      "function foo() {
      if (a) {
          return
          b();
          c();
      } else {
          throw err
          d();
      }
  }",
      vec![(4, 10), (5, 10), (8, 10)],
    );
  }

  #[test]
  fn err_9() {
    assert_lint_err_on_line_n::<NoUnreachable>(
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
  }",
      vec![(4, 10), (5, 10), (8, 10), (10, 6)],
    );

    assert_lint_err_on_line::<NoUnreachable>(
      "function* foo() {
      try {
          return;
      } catch (err) {
          return err;
      }
  }",
      5,
      10,
    );

    assert_lint_err_on_line::<NoUnreachable>(
      "function foo() {
      try {
          return;
      } catch (err) {
          return err;
      }
  }",
      5,
      10,
    );
  }

  #[test]
  fn err_10() {
    assert_lint_err_on_line_n::<NoUnreachable>(
      "function foo() {
      try {
          return;
          let a = 1;
      } catch (err) {
          return err;
      }
  }",
      vec![(4, 10), (6, 10)],
    );
  }

  #[test]
  fn deno_ok_1() {
    assert_lint_ok::<NoUnreachable>(
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
    )
  }

  #[test]
  fn issue_340_1() {
    assert_lint_ok::<NoUnreachable>(
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
    );
  }

  #[test]
  fn issue_340_2() {
    assert_lint_ok::<NoUnreachable>(
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
    );
  }

  #[test]
  fn issue_340_3() {
    assert_lint_ok::<NoUnreachable>(
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
    );
  }
}
