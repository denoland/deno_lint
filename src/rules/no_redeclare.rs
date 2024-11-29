// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::tags::{self, Tags};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::Id;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::{
  ast::*, utils::find_pat_ids, visit::Visit, visit::VisitWith,
};
use deno_ast::SourceRangedForSpanned;

use std::collections::HashSet;

#[derive(Debug)]
pub struct NoRedeclare;

const CODE: &str = "no-redeclare";
const MESSAGE: &str = "Redeclaration is not allowed";

impl LintRule for NoRedeclare {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoRedeclareVisitor {
      context,
      bindings: Default::default(),
    };
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m),
      ProgramRef::Script(s) => visitor.visit_script(s),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_redeclare.md")
  }
}

struct NoRedeclareVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  /// TODO(kdy1): Change this to HashMap<Id, Vec<SourceRange>> and use those ranges to point previous bindings/
  bindings: HashSet<Id>,
}

impl<'c, 'view> NoRedeclareVisitor<'c, 'view> {
  fn declare(&mut self, i: &Ident) {
    let id = i.to_id();

    if !self.bindings.insert(id) {
      self.context.add_diagnostic(i.range(), CODE, MESSAGE);
    }
  }
}

impl<'c, 'view> Visit for NoRedeclareVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, f: &FnDecl) {
    if f.function.body.is_none() {
      return;
    }

    self.declare(&f.ident);

    f.visit_children_with(self);
  }

  fn visit_var_declarator(&mut self, v: &VarDeclarator) {
    let ids: Vec<Ident> = find_pat_ids(&v.name);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_param(&mut self, p: &Param) {
    let ids: Vec<Ident> = find_pat_ids(&p.pat);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_class_prop(&mut self, p: &ClassProp) {
    if let PropName::Computed(_) = &p.key {
      p.key.visit_with(self);
    }

    p.value.visit_with(self);
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
      "function f(foo: number, foo: string) {}": [{line: 1, col: 24, message: MESSAGE}],
    }
  }
}
