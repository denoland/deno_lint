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
use std::sync::Arc;

pub struct NoUndef;

impl LintRule for NoUndef {
  fn new() -> Box<Self> {
    Box::new(NoUndef)
  }

  fn code(&self) -> &'static str {
    "no-undef"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
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

struct NoUndefVisitor {
  context: Arc<Context>,
  declared: HashSet<Id>,
}

impl NoUndefVisitor {
  fn new(context: Arc<Context>, declared: HashSet<Id>) -> Self {
    Self { context, declared }
  }

  fn check(&self, ident: &Ident) {
    // Thanks to this if statement, we can check for Map in
    //
    // function foo(Map) { ... }
    //
    if ident.span.ctxt != self.context.top_level_ctxt {
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
      &format!("{} is not defined", ident.sym),
    )
  }
}

impl Visit for NoUndefVisitor {
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
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoUndef>("var a = 1, b = 2; a;");

    assert_lint_ok::<NoUndef>("function a(){}  a();");

    assert_lint_ok::<NoUndef>("function f(b) { b; }");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoUndef>("var a; a = 1; a++;");

    assert_lint_ok::<NoUndef>("var a; function f() { a = 1; }");

    assert_lint_ok::<NoUndef>("Object; isNaN();");
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoUndef>("toString()");

    assert_lint_ok::<NoUndef>("hasOwnProperty()");

    assert_lint_ok::<NoUndef>("function evilEval(stuffToEval) { var ultimateAnswer; ultimateAnswer = 42; eval(stuffToEval); }");
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoUndef>("typeof a");

    assert_lint_ok::<NoUndef>("typeof (a)");

    assert_lint_ok::<NoUndef>("var b = typeof a");
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoUndef>("typeof a === 'undefined'");

    assert_lint_ok::<NoUndef>("if (typeof a === 'undefined') {}");

    assert_lint_ok::<NoUndef>(
      "function foo() { var [a, b=4] = [1, 2]; return {a, b}; }",
    );
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoUndef>("var toString = 1;");

    assert_lint_ok::<NoUndef>("function myFunc(...foo) {  return foo;}");

    // TODO(kdy1): Parse as jsx
    // assert_lint_ok::<NoUndef>(
    //   "var React, App, a=1; React.render(<App attr={a} />);",
    // );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoUndef>(
      "var console; [1,2,3].forEach(obj => {\n  console.log(obj);\n});",
    );

    assert_lint_ok::<NoUndef>(
      "var Foo; class Bar extends Foo { constructor() { super();  }}",
    );

    assert_lint_ok::<NoUndef>(
      "import Warning from '../lib/warning'; var warn = new Warning('text');",
    );
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoUndef>("import * as Warning from '../lib/warning'; var warn = new Warning('text');");

    assert_lint_ok::<NoUndef>("var a; [a] = [0];");

    assert_lint_ok::<NoUndef>("var a; ({a} = {});");
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoUndef>("var a; ({b: a} = {});");

    assert_lint_ok::<NoUndef>("var obj; [obj.a, obj.b] = [0, 1];");

    assert_lint_ok::<NoUndef>(
      "(foo, bar) => { foo ||= WeakRef; bar ??= FinalizationRegistry; }",
    );
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoUndef>("Array = 1;");

    assert_lint_ok::<NoUndef>("class A { constructor() { new.target; } }");

    assert_lint_ok::<NoUndef>(r#"export * as ns from "source""#);
  }

  #[test]
  fn ok_11() {
    assert_lint_ok::<NoUndef>("import.meta");

    assert_lint_ok::<NoUndef>(
      "
      await new Promise((resolve: () => void, _) => {
        setTimeout(resolve, 100);
      });
      ",
    );
  }

  #[test]
  fn ok_12() {
    assert_lint_ok::<NoUndef>(
      "
      const importPath = \"./foo.ts\";
      const dataProcessor = await import(importPath);
      ",
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoUndef>("a = 1;", 0);

    assert_lint_err::<NoUndef>("var a = b;", 8);

    assert_lint_err::<NoUndef>("function f() { b; }", 15);
  }

  #[test]
  fn err_2() {
    // assert_lint_err::<NoUndef>("var React; React.render(<img attr={a} />);", 0);
  }

  #[test]
  fn err_3() {
    // assert_lint_err::<NoUndef>(
    //   "var React, App; React.render(<App attr={a} />);",
    //   0,
    // );

    assert_lint_err::<NoUndef>("[a] = [0];", 1);

    assert_lint_err::<NoUndef>("({a} = {});", 2);
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoUndef>("({b: a} = {});", 5);

    assert_lint_err_n::<NoUndef>("[obj.a, obj.b] = [0, 1];", vec![1, 8]);

    assert_lint_err::<NoUndef>("const c = 0; const a = {...b, c};", 27);
  }

  #[test]
  fn deno_ok_1() {
    assert_lint_ok::<NoUndef>(
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
    );
  }

  #[test]
  fn deno_ok_2() {
    assert_lint_ok::<NoUndef>(
      r#"
    const listeners = [];
    for (const listener of listeners) {
      try {
      } catch (err) {
        this.emit("error", err);
      }
    }
    "#,
    );
  }
}
