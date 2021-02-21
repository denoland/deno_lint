// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::{
  ast::*, utils::find_ids, utils::ident::IdentLike, utils::Id, visit::Node,
  visit::Visit, visit::VisitWith,
};

use std::collections::HashSet;

pub struct NoRedeclare;

const CODE: &str = "no-redeclare";
const MESSAGE: &str = "Redeclaration is not allowed";

impl LintRule for NoRedeclare {
  fn new() -> Box<Self> {
    Box::new(NoRedeclare)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = NoRedeclareVisitor {
      context,
      bindings: Default::default(),
    };
    program.visit_with(program, &mut visitor);
  }
}

struct NoRedeclareVisitor<'c> {
  context: &'c mut Context,
  /// TODO(kdy1): Change this to HashMap<Id, Vec<Span>> and use those spans to point previous bindings/
  bindings: HashSet<Id>,
}

impl<'c> NoRedeclareVisitor<'c> {
  fn declare(&mut self, i: &Ident) {
    let id = i.to_id();

    if !self.bindings.insert(id) {
      self.context.add_diagnostic(i.span, CODE, MESSAGE);
    }
  }
}

impl<'c> Visit for NoRedeclareVisitor<'c> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, f: &FnDecl, _: &dyn Node) {
    if f.function.body.is_none() {
      return;
    }

    self.declare(&f.ident);

    f.visit_children_with(self);
  }

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&v.name);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_param(&mut self, p: &Param, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&p.pat);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_class_prop(&mut self, p: &ClassProp, _: &dyn Node) {
    if p.computed {
      p.key.visit_with(p, self);
    }

    p.value.visit_with(p, self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_redeclare_valid() {
    assert_lint_ok! {
      NoRedeclare,
      "var a = 3; var b = function() { var a = 10; };",
      "var a = 3; a = 10;",
      "if (true) {\n    let b = 2;\n} else {    \nlet b = 3;\n}",
      "class C {
        constructor(a: string) {}
      }
      class D {
        constructor(a: string) {}
      }",

      // https://github.com/denoland/deno_lint/issues/615
      "class T { #foo(x) {} #bar(x) {} }",
    };
  }

  #[test]
  fn no_redeclare_invalid() {
    assert_lint_err! {
      NoRedeclare,
      "var a = 3; var a = 10;": [{col: 15, message: MESSAGE}],
      "switch(foo) { case a: var b = 3;\ncase b: var b = 4}": [{col: 12, line: 2, message: MESSAGE}],
      "var a = 3; var a = 10;": [{col: 15, message: MESSAGE}],
      "var a = {}; var a = [];": [{col: 16, message: MESSAGE}],
      "var a; function a() {}": [{col: 16, message: MESSAGE}],
      "function a() {} function a() {}": [{col: 25, message: MESSAGE}],
      "var a = function() { }; var a = function() { }": [{col: 28, message: MESSAGE}],
      "var a = function() { }; var a = new Date();": [{col: 28, message: MESSAGE}],
      "var a; var a;": [{col: 11, message: MESSAGE}],
      "export var a; var a;": [{col: 18, message: MESSAGE}],
      "function f() { var a; var a; }": [{col: 26, message: MESSAGE}],
      "function f(a) { var a; }": [{col: 20, message: MESSAGE}],
      "function f() { var a; if (test) { var a; } }": [{col: 38, message: MESSAGE}],
      "for (var a, a;;);": [{col: 12, message: MESSAGE}],
      "let a; let a;": [{col: 11, message: MESSAGE}],
      "let a; const a = 0;": [{col: 13, message: MESSAGE}],
      "const a = 0; const a = 0;": [{col: 19, message: MESSAGE}],
      "if (test) { let a; let a; }": [{col: 23, message: MESSAGE}],
      "switch (test) { case 0: let a; let a; }": [{col: 35, message: MESSAGE}],
      "for (let a, a;;);": [{col: 12, message: MESSAGE}],
      "for (let [a, a] in xs);": [{col: 13, message: MESSAGE}],
      "function f() { let a; let a; }": [{col: 26, message: MESSAGE}],
      "function f(a) { let a; }": [{col: 20, message: MESSAGE}],
      "function f() { if (test) { let a; let a; } }": [{col: 38, message: MESSAGE}],
      "var a = 3; var a = 10; var a = 15;": [{col: 15, message: MESSAGE}, {col: 27, message: MESSAGE}],
      "var a; var {a = 0, b: Object = 0} = {};": [{line: 1, col: 12, message: MESSAGE}],
      "var a; var {a = 0, b: globalThis = 0} = {};": [{line: 1, col: 12, message: MESSAGE}],
    }
  }
}
