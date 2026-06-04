// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::globals::GLOBALS;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::syntax::operator::UnaryOperator;
use deno_ast::oxc::syntax::scope::ScopeFlags;

#[derive(Debug)]
pub struct NoUndef;

impl LintRule for NoUndef {
  fn code(&self) -> &'static str {
    "no-undef"
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut visitor = NoUndefVisitor {
      context,
      in_typeof: false,
      type_context_depth: 0,
    };
    visitor.visit_program(program);
  }
}

struct NoUndefVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  in_typeof: bool,
  /// Tracks how deep we are inside a type annotation context.
  /// When > 0, identifier references are type-level and should not be checked.
  type_context_depth: u32,
}

impl NoUndefVisitor<'_, '_> {
  fn check(&mut self, ident: &IdentifierReference) {
    if self.type_context_depth > 0 {
      return;
    }

    if self.in_typeof {
      return;
    }

    // Implicitly defined
    // See: https://github.com/denoland/deno_lint/issues/317
    if ident.name == "arguments" {
      return;
    }

    // Use OXC's semantic analysis to check if the identifier is resolved.
    // `reference_id` is set during semantic analysis; if the reference resolves
    // to a symbol, it's defined. If it doesn't, OXC leaves `symbol_id` as None.
    if let Some(ref_id) = ident.reference_id.get() {
      let reference = self.context.scoping().get_reference(ref_id);
      if reference.symbol_id().is_some() {
        // Resolved to a local binding
        return;
      }
    }

    // Globals
    if GLOBALS.iter().any(|(name, _)| *name == ident.name.as_str()) {
      return;
    }

    self.context.add_diagnostic(
      ident.span,
      "no-undef",
      format!("{} is not defined", ident.name),
    )
  }
}

impl<'a> Visit<'a> for NoUndefVisitor<'_, 'a> {
  fn visit_identifier_reference(&mut self, ident: &IdentifierReference<'a>) {
    self.check(ident);
  }

  fn visit_unary_expression(&mut self, expr: &UnaryExpression<'a>) {
    if expr.operator == UnaryOperator::Typeof {
      let prev = self.in_typeof;
      self.in_typeof = true;
      walk::walk_unary_expression(self, expr);
      self.in_typeof = prev;
    } else {
      walk::walk_unary_expression(self, expr);
    }
  }

  fn visit_ts_type_annotation(&mut self, annotation: &TSTypeAnnotation<'a>) {
    self.type_context_depth += 1;
    walk::walk_ts_type_annotation(self, annotation);
    self.type_context_depth -= 1;
  }

  fn visit_ts_type_parameter_declaration(
    &mut self,
    decl: &TSTypeParameterDeclaration<'a>,
  ) {
    self.type_context_depth += 1;
    walk::walk_ts_type_parameter_declaration(self, decl);
    self.type_context_depth -= 1;
  }

  fn visit_ts_type_parameter_instantiation(
    &mut self,
    inst: &TSTypeParameterInstantiation<'a>,
  ) {
    self.type_context_depth += 1;
    walk::walk_ts_type_parameter_instantiation(self, inst);
    self.type_context_depth -= 1;
  }

  fn visit_ts_type_assertion(&mut self, assertion: &TSTypeAssertion<'a>) {
    // Only the expression part is runtime, the type annotation is a type context
    self.type_context_depth += 1;
    walk::walk_ts_type(self, &assertion.type_annotation);
    self.type_context_depth -= 1;
    self.visit_expression(&assertion.expression);
  }

  fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
    self.visit_expression(&expr.expression);
    self.type_context_depth += 1;
    walk::walk_ts_type(self, &expr.type_annotation);
    self.type_context_depth -= 1;
  }

  fn visit_ts_satisfies_expression(
    &mut self,
    expr: &TSSatisfiesExpression<'a>,
  ) {
    self.visit_expression(&expr.expression);
    self.type_context_depth += 1;
    walk::walk_ts_type(self, &expr.type_annotation);
    self.type_context_depth -= 1;
  }

  fn visit_ts_type_alias_declaration(
    &mut self,
    decl: &TSTypeAliasDeclaration<'a>,
  ) {
    // The type alias itself is in a type context
    self.type_context_depth += 1;
    walk::walk_ts_type_alias_declaration(self, decl);
    self.type_context_depth -= 1;
  }

  fn visit_ts_interface_declaration(
    &mut self,
    decl: &TSInterfaceDeclaration<'a>,
  ) {
    // Interface body is a type context
    self.type_context_depth += 1;
    walk::walk_ts_interface_declaration(self, decl);
    self.type_context_depth -= 1;
  }

  fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
    walk::walk_function(self, func, flags);
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
