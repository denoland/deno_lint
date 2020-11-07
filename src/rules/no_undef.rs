// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use std::collections::HashSet;

use super::Context;
use crate::globals::GLOBALS;
use crate::scoped_rule::ScopeRule;
use crate::scoped_rule::ScopedRule;
use crate::scopes::BindingKind;
use swc_ecmascript::utils::Id;
use swc_ecmascript::{ast::*, utils::ident::IdentLike};

pub type NoUndef = ScopedRule<NoUndefImpl>;

#[derive(Default)]
pub struct NoUndefImpl {
  /// declared bindings
  bindings: HashSet<Id>,
}

impl ScopeRule for NoUndefImpl {
  fn new() -> Self {
    NoUndefImpl::default()
  }

  fn tags() -> &'static [&'static str] {
    &["recommended"]
  }

  fn code() -> &'static str {
    "no-undef"
  }

  fn ignore_typeof() -> bool {
    true
  }

  fn declare(&mut self, _: &mut Context, i: &Ident, _: BindingKind) {
    self.bindings.insert(i.to_id());
  }

  fn assign(&mut self, context: &mut Context, i: &Ident) {
    self.check_usage(context, i)
  }

  fn check_usage(&mut self, context: &mut Context, i: &Ident) {
    // Thanks to this if statement, we can check for Map in
    //
    // function foo(Map) { ... }
    //
    if i.span.ctxt != context.top_level_ctxt {
      return;
    }

    // Implicitly defined
    // See: https://github.com/denoland/deno_lint/issues/317
    if i.sym == *"arguments" {
      return;
    }

    // Ignore top level bindings declared in the file.
    if self.bindings.contains(&i.to_id()) {
      return;
    }

    // Globals
    if GLOBALS.iter().any(|(name, _)| name == &&*i.sym) {
      return;
    }

    context.add_diagnostic(
      i.span,
      "no-undef",
      format!("{} is not defined", i.sym),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_undef_valid() {
    assert_lint_ok! {
      NoUndef,
      "var a = 1, b = 2; a;",
      "function a(){}  a();",
      "function f(b) { b; }",
      "var a; a = 1; a++;",
      "var a; function f() { a = 1; }",
      "Object; isNaN();",
      "toString()",
      "hasOwnProperty()",
      "function evilEval(stuffToEval) { var ultimateAnswer; ultimateAnswer = 42; eval(stuffToEval); }",
      "typeof a",
      "typeof (a)",
      "var b = typeof a",
      "typeof a === 'undefined'",
      "if (typeof a === 'undefined') {}",
      "function foo() { var [a, b=4] = [1, 2]; return {a, b}; }",
      "var toString = 1;",
      "function myFunc(...foo) {  return foo;}",
      "function myFunc() { console.log(arguments); }",
      // TODO(kdy1): Parse as jsx
      // "var React, App, a=1; React.render(<App attr={a} />);",
      "var console; [1,2,3].forEach(obj => {\n  console.log(obj);\n});",
      "var Foo; class Bar extends Foo { constructor() { super();  }}",
      "import Warning from '../lib/warning'; var warn = new Warning('text');",
      "import * as Warning from '../lib/warning'; var warn = new Warning('text');",
      "var a; [a] = [0];",
      "var a; ({a} = {});",
      "var a; ({b: a} = {});",
      "var obj; [obj.a, obj.b] = [0, 1];",
      "(foo, bar) => { foo ||= WeakRef; bar ??= FinalizationRegistry; }",
      "Array = 1;",
      "class A { constructor() { new.target; } }",
      r#"export * as ns from "source""#,
      "import.meta",
      "
      await new Promise((resolve: () => void, _) => {
        setTimeout(resolve, 100);
      });
      ",
      "
      const importPath = \"./foo.ts\";
      const dataProcessor = await import(importPath);
      ",
      r#"
    class PartWriter implements Deno.Writer {
      closed = false;
      private readonly partHeader: string;
      private headersWritten = false;

      constructor(
        private writer: Deno.Writer,
        readonly boundary: string,
        public headers: Headers,
        isFirstBoundary: boolean,
      ) {
        let buf = "";
        if (isFirstBoundary) {
          buf += `--${boundary}\r\n`;
        } else {
          buf += `\r\n--${boundary}\r\n`;
        }
        for (const [key, value] of headers.entries()) {
          buf += `${key}: ${value}\r\n`;
        }
        buf += `\r\n`;
        this.partHeader = buf;
      }

      close(): void {
        this.closed = true;
      }

      async write(p: Uint8Array): Promise<number> {
        if (this.closed) {
          throw new Error("part is closed");
        }
        if (!this.headersWritten) {
          this.headersWritten = true;
        }
        return this.writer.write(p);
      }
    }
    "#,
      r#"
    const listeners = [];
    for (const listener of listeners) {
      try {
      } catch (err) {
        this.emit("error", err);
      }
    }
    "#,

      // https://github.com/denoland/deno_lint/issues/463
      r#"
    (() => {
      function foo() {
        return new Bar();
      }
      class Bar {}
    })();
        "#,
      r#"
    function f() {
      function foo() {
        return new Bar();
      }
      class Bar {}
    }
    "#,
      r#"
    function f() {
      foo++;
      {
        var foo = 1;
      }
    }
    "#,
    };
  }

  #[test]
  fn no_undef_invalid() {
    assert_lint_err! {
      NoUndef,
      "a = 1;": [
        {
          col: 0,
          message: "a is not defined",
        },
      ],
      "var a = b;": [
        {
          col: 8,
          message: "b is not defined",
        },
      ],
      "function f() { b; }": [
        {
          col: 15,
          message: "b is not defined",
        },
      ],
      // "var React; React.render(<img attr={a} />);": [
      //   {
      //     col: 0,
      //     message: "a is not defined",
      //    },
      // ],
      // "var React, App; React.render(<App attr={a} />);": [
      //   {
      //     col: 0,
      //     message: "a is not defined",
      //   },
      // ],
      "[a] = [0];": [
        {
          col: 1,
          message: "a is not defined",
        },
      ],
      "({a} = {});": [
        {
          col: 2,
          message: "a is not defined",
        },
      ],
      "({b: a} = {});": [
        {
          col: 5,
          message: "a is not defined",
        },
      ],
      "[obj.a, obj.b] = [0, 1];": [
        {
          col: 1,
          message: "obj is not defined",
        },
        {
          col: 8,
          message: "obj is not defined",
        },
      ],
      "const c = 0; const a = {...b, c};": [
        {
          col: 27,
          message: "b is not defined",
        },
      ],
      "foo++; function f() { var foo = 0; }": [
        {
          col: 0,
          message: "foo is not defined",
        },
      ],
      "foo++; { let foo = 0; }": [
        {
          col: 0,
          message: "foo is not defined",
        },
      ],
    };
  }
}
