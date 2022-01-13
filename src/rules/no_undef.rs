// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::globals::GLOBALS;
use crate::ProgramRef;
use deno_ast::swc::{
  ast::*,
  utils::ident::IdentLike,
  visit::{noop_visit_type, Visit, VisitWith},
};
use std::sync::Arc;

#[derive(Debug)]
pub struct NoUndef;

impl LintRule for NoUndef {
  fn new() -> Arc<Self> {
    Arc::new(NoUndef)
  }

  fn code(&self) -> &'static str {
    "no-undef"
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoUndefVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_undef.md")
  }
}

struct NoUndefVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoUndefVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check(&mut self, ident: &Ident) {
    // Thanks to this if statement, we can check for Map in
    //
    // function foo(Map) { ... }
    //
    if ident.span.ctxt != self.context.top_level_ctxt() {
      return;
    }

    // Implicitly defined
    // See: https://github.com/denoland/deno_lint/issues/317
    if ident.sym == *"arguments" {
      return;
    }

    if self.context.scope().var(&ident.to_id()).is_some() {
      return;
    }

    // Globals
    if GLOBALS.iter().any(|(name, _)| name == &&*ident.sym) {
      return;
    }

    self.context.add_diagnostic(
      ident.span,
      "no-undef",
      format!("{} is not defined", ident.sym),
    )
  }
}

impl<'c, 'view> Visit for NoUndefVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_member_expr(&mut self, e: &MemberExpr) {
    e.obj.visit_with(self);
    if let MemberProp::Computed(prop) = &e.prop {
      prop.visit_with(self);
    }
  }

  fn visit_unary_expr(&mut self, e: &UnaryExpr) {
    if e.op == UnaryOp::TypeOf {
      return;
    }

    e.visit_children_with(self);
  }

  fn visit_expr(&mut self, e: &Expr) {
    e.visit_children_with(self);

    if let Expr::Ident(ident) = e {
      self.check(ident)
    }
  }

  fn visit_class_prop(&mut self, p: &ClassProp) {
    p.value.visit_with(self)
  }

  fn visit_prop(&mut self, p: &Prop) {
    p.visit_children_with(self);

    if let Prop::Shorthand(i) = &p {
      self.check(i);
    }
  }

  fn visit_pat(&mut self, p: &Pat) {
    if let Pat::Ident(i) = p {
      self.check(&i.id);
    } else {
      p.visit_children_with(self);
    }
  }

  fn visit_assign_pat_prop(&mut self, p: &AssignPatProp) {
    self.check(&p.key);
    p.value.visit_with(self);
  }

  fn visit_call_expr(&mut self, e: &CallExpr) {
    if let Callee::Import(_) = &e.callee {
      return;
    }

    e.visit_children_with(self)
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

      // https://github.com/denoland/deno_lint/issues/607
      r#"type Foo = ""; export default Foo;"#,
      r#"type Foo = Array<string>; export default Foo;"#,
      r#"type Foo = string | number; export default Foo;"#,
      r#"type Foo<T> = { bar: T }; export default Foo;"#,
      r#"type Foo = string | undefined; export type { Foo };"#,

      // https://github.com/denoland/deno_lint/issues/596
      r#"
      const f = (
        { a }: Foo,
        b: boolean,
      ) => {};
      "#,

      // https://github.com/denoland/deno_lint/issues/643
      "export default function foo() {} foo();",
      "export default class Foo {} const foo = new Foo();",
      "export default interface Foo {} const foo: Foo = {};",

      // https://github.com/denoland/deno_lint/issues/658
      r#"function foo([nb, min]: [number, number], [value, diff]: [number, number]) { return "Hello Bug !" }"#,
      r#"const foo = ([nb, min]: [number, number], [value, diff]: [number, number]) => "Hello Bug !""#,
      "function foo([a]: [number], [b]: [boolean]) {}",
      "function foo([a, x]: [number, number], [b]: [boolean]) {}",
      "function foo([a]: [number], [b, y]: [boolean, boolean]) {}",
      "function foo({ a }: { a: number }, [b]: [boolean]) {}",
      "const foo = ([a]: [number], [b]: [boolean]) => {};",
      "const foo = ([a, x]: [number, number], [b]: [boolean]) => {};",
      "const foo = ([a]: [number], [b, y]: [boolean, boolean]) => {};",
      "const foo = ({ a }: { a: number }, [b]: [boolean]) => {};",
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
      "type Foo = string; export default Bar;": [
        {
          col: 34,
          message: "Bar is not defined",
        },
      ],
    };
  }
}
