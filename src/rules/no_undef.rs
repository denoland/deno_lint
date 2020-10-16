// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::globals::GLOBALS;
use swc_atoms::js_word;
use swc_common::SyntaxContext;
use swc_ecmascript::{
  ast::*,
  utils::ident::IdentLike,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};
use swc_ecmascript::{utils::find_ids, utils::Id};

use std::collections::HashSet;

pub struct NoUndef;

impl LintRule for NoUndef {
  fn new() -> Box<Self> {
    Box::new(NoUndef)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-undef"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut collector = BindingCollector {
      top_level_ctxt: context.top_level_ctxt,
      declared: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoUndefVisitor::new(context, collector.declared);
    module.visit_with(module, &mut visitor);
  }
}

/// Collects top level bindings, which have top level syntax
/// context passed to the resolver.
struct BindingCollector {
  /// Optimization. Unresolved references and top
  /// level bindings will have this context.
  top_level_ctxt: SyntaxContext,

  /// If there exists a binding with such id, it's not global.
  declared: HashSet<Id>,
}

impl BindingCollector {
  fn declare(&mut self, i: Id) {
    if i.1 != self.top_level_ctxt {
      return;
    }
    self.declared.insert(i);
  }
}

impl Visit for BindingCollector {
  fn visit_fn_decl(&mut self, f: &FnDecl, _: &dyn Node) {
    self.declare(f.ident.to_id());
  }

  fn visit_class_decl(&mut self, f: &ClassDecl, _: &dyn Node) {
    self.declare(f.ident.to_id());
  }

  fn visit_class_expr(&mut self, n: &ClassExpr, _: &dyn Node) {
    if let Some(i) = &n.ident {
      self.declare(i.to_id());
    }
  }

  fn visit_import_named_specifier(
    &mut self,
    i: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.declare(i.local.to_id());
  }

  fn visit_import_default_specifier(
    &mut self,
    i: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.declare(i.local.to_id());
  }

  fn visit_import_star_as_specifier(
    &mut self,
    i: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.declare(i.local.to_id());
  }

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    let ids: Vec<Id> = find_ids(&v.name);
    for id in ids {
      self.declare(id);
    }
  }

  fn visit_ts_enum_decl(&mut self, e: &TsEnumDecl, _: &dyn Node) {
    self.declare(e.id.to_id());
  }

  fn visit_ts_param_prop_param(&mut self, p: &TsParamPropParam, _: &dyn Node) {
    match p {
      TsParamPropParam::Ident(i) => {
        self.declare(i.to_id());
      }
      TsParamPropParam::Assign(i) => {
        let ids: Vec<Id> = find_ids(&i.left);
        for id in ids {
          self.declare(id);
        }
      }
    }
  }

  fn visit_param(&mut self, p: &Param, _: &dyn Node) {
    let ids: Vec<Id> = find_ids(&p.pat);
    for id in ids {
      self.declare(id);
    }
  }
  fn visit_catch_clause(&mut self, c: &CatchClause, _: &dyn Node) {
    if let Some(pat) = &c.param {
      let ids: Vec<Id> = find_ids(pat);
      for id in ids {
        self.declare(id);
      }
    }

    c.body.visit_with(c, self);
  }
}

struct NoUndefVisitor<'c> {
  context: &'c mut Context,
  declared: HashSet<Id>,
}

impl<'c> NoUndefVisitor<'c> {
  fn new(context: &'c mut Context, declared: HashSet<Id>) -> Self {
    Self { context, declared }
  }

  fn check(&mut self, ident: &Ident) {
    // Thanks to this if statement, we can check for Map in
    //
    // function foo(Map) { ... }
    //
    if ident.span.ctxt != self.context.top_level_ctxt {
      return;
    }

    // Implicitly defined
    // See: https://github.com/denoland/deno_lint/issues/317
    if ident.sym == *"arguments" {
      return;
    }

    // Ignore top level bindings declared in the file.
    if self.declared.contains(&ident.to_id()) {
      return;
    }

    // Globals
    if GLOBALS.contains(&&*ident.sym) {
      return;
    }

    self.context.add_diagnostic(
      ident.span,
      "no-undef",
      format!("{} is not defined", ident.sym),
    )
  }
}

impl<'c> Visit for NoUndefVisitor<'c> {
  noop_visit_type!();

  fn visit_member_expr(&mut self, e: &MemberExpr, _: &dyn Node) {
    e.obj.visit_with(e, self);
    if e.computed {
      e.prop.visit_with(e, self);
    }
  }

  fn visit_unary_expr(&mut self, e: &UnaryExpr, _: &dyn Node) {
    if e.op == UnaryOp::TypeOf {
      return;
    }

    e.visit_children_with(self);
  }

  fn visit_expr(&mut self, e: &Expr, _: &dyn Node) {
    e.visit_children_with(self);

    if let Expr::Ident(ident) = e {
      self.check(ident)
    }
  }

  fn visit_class_prop(&mut self, p: &ClassProp, _: &dyn Node) {
    p.value.visit_with(p, self)
  }

  fn visit_prop(&mut self, p: &Prop, _: &dyn Node) {
    p.visit_children_with(self);

    if let Prop::Shorthand(i) = &p {
      self.check(i);
    }
  }

  fn visit_pat(&mut self, p: &Pat, _: &dyn Node) {
    if let Pat::Ident(i) = p {
      self.check(i);
    } else {
      p.visit_children_with(self);
    }
  }

  fn visit_assign_pat_prop(&mut self, p: &AssignPatProp, _: &dyn Node) {
    self.check(&p.key);
    p.value.visit_with(p, self);
  }

  fn visit_call_expr(&mut self, e: &CallExpr, _: &dyn Node) {
    if let ExprOrSuper::Expr(callee) = &e.callee {
      if let Expr::Ident(i) = &**callee {
        if i.sym == js_word!("import") {
          return;
        }
      }
    }

    e.visit_children_with(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ok_1() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "var a = 1, b = 2; a;",
        "function a(){}  a();",
        "function f(b) { b; }",
      ],
    };
  }

  #[test]
  fn ok_2() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "var a; a = 1; a++;",
        "var a; function f() { a = 1; }",
        "Object; isNaN();",
      ],
    };
  }

  #[test]
  fn ok_3() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "toString()",
        "hasOwnProperty()",
        "function evilEval(stuffToEval) { var ultimateAnswer; ultimateAnswer = 42; eval(stuffToEval); }",
      ],
    };
  }

  #[test]
  fn ok_4() {
    assert_lint_ok_macro! {
      NoUndef,
      ["typeof a", "typeof (a)", "var b = typeof a",]
    };
  }

  #[test]
  fn ok_5() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "typeof a === 'undefined'",
        "if (typeof a === 'undefined') {}",
        "function foo() { var [a, b=4] = [1, 2]; return {a, b}; }",
      ],
    };
  }

  #[test]
  fn ok_6() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "var toString = 1;",
        "function myFunc(...foo) {  return foo;}",
        "function myFunc() { console.log(arguments); }",
      ],
    };

    // TODO(kdy1): Parse as jsx
    // assert_lint_ok::<NoUndef>(
    //   "var React, App, a=1; React.render(<App attr={a} />);",
    // );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "var console; [1,2,3].forEach(obj => {\n  console.log(obj);\n});",
        "var Foo; class Bar extends Foo { constructor() { super();  }}",
        "import Warning from '../lib/warning'; var warn = new Warning('text');",
      ],
    };
  }

  #[test]
  fn ok_8() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "import * as Warning from '../lib/warning'; var warn = new Warning('text');",
        "var a; [a] = [0];",
        "var a; ({a} = {});",
      ],
    };
  }

  #[test]
  fn ok_9() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "var a; ({b: a} = {});",
        "var obj; [obj.a, obj.b] = [0, 1];",
        "(foo, bar) => { foo ||= WeakRef; bar ??= FinalizationRegistry; }",
      ],
    };
  }

  #[test]
  fn ok_10() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "Array = 1;",
        "class A { constructor() { new.target; } }",
        r#"export * as ns from "source""#,
      ],
    };
  }

  #[test]
  fn ok_11() {
    assert_lint_ok_macro! {
      NoUndef,
      [
        "import.meta",
        "
        await new Promise((resolve: () => void, _) => {
          setTimeout(resolve, 100);
        });
        ",
      ],
    };
  }

  #[test]
  fn ok_12() {
    assert_lint_ok_macro! {
      NoUndef,
      "
      const importPath = \"./foo.ts\";
      const dataProcessor = await import(importPath);
      ",
    };
  }

  #[test]
  fn err_1() {
    assert_lint_err_macro! {
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
    };
  }

  #[test]
  fn err_2() {
    // assert_lint_err_macro! {
    //   NoUndef,
    //   "var React; React.render(<img attr={a} />);": [
    //     {
    //       col: 0,
    //       message: "a is not defined",
    //      },
    //   ],
    // };
  }

  #[test]
  fn err_3() {
    assert_lint_err_macro! {
      NoUndef,
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
    };
  }

  #[test]
  fn err_4() {
    assert_lint_err_macro! {
      NoUndef,
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
    };
  }

  #[test]
  fn deno_ok_1() {
    assert_lint_ok_macro! {
      NoUndef,
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
    };
  }

  #[test]
  fn deno_ok_2() {
    assert_lint_ok_macro! {
      NoUndef,
      r#"
    const listeners = [];
    for (const listener of listeners) {
      try {
      } catch (err) {
        this.emit("error", err);
      }
    }
    "#,
    };
  }
}
