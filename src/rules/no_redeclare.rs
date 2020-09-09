// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::globals::GLOBALS;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::{
  ast::*, utils::find_ids, utils::ident::IdentLike, utils::Id, visit::Node,
  visit::Visit, visit::VisitWith,
};

use std::collections::HashSet;
use std::sync::Arc;

pub struct NoRedeclare;

impl LintRule for NoRedeclare {
  fn new() -> Box<Self> {
    Box::new(NoRedeclare)
  }

  fn code(&self) -> &'static str {
    "no-redeclare"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoRedeclareVisitor {
      context,
      bindings: Default::default(),
    };
    module.visit_with(module, &mut visitor);
  }
}

struct NoRedeclareVisitor {
  context: Arc<Context>,
  /// TODO(kdy1): Change this to HashMap<Id, Vec<Span>> and use those spans to point previous bindings/
  bindings: HashSet<Id>,
}

impl NoRedeclareVisitor {
  fn declare(&mut self, i: &Ident) {
    let id = i.to_id();

    if !self.bindings.insert(id) || GLOBALS.contains(&&*i.sym) {
      self.context.add_diagnostic(
        i.span,
        "no-redeclare",
        "Redeclaration is not allowed",
      );
    }
  }
}

impl Visit for NoRedeclareVisitor {
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

  fn visit_for_stmt(&mut self, n: &ForStmt, _: &dyn Node) {
    n.test.visit_with(n, self);
    n.update.visit_with(n, self);
    n.body.visit_with(n, self);
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    n.right.visit_with(n, self);
    n.body.visit_with(n, self);
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    n.right.visit_with(n, self);
    n.body.visit_with(n, self);
  }

  fn visit_param(&mut self, p: &Param, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&p.pat);

    for id in ids {
      self.declare(&id);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoRedeclare>(
      "var a = 3; var b = function() { var a = 10; };",
    );

    assert_lint_ok::<NoRedeclare>("var a = 3; a = 10;");

    assert_lint_ok::<NoRedeclare>(
      "if (true) {\n    let b = 2;\n} else {    \nlet b = 3;\n}",
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 15);

    assert_lint_err_on_line::<NoRedeclare>(
      "switch(foo) { case a: var b = 3;\ncase b: var b = 4}",
      2,
      12,
    );

    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 15);
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoRedeclare>("var a = {}; var a = [];", 16);

    assert_lint_err::<NoRedeclare>("var a; function a() {}", 16);

    assert_lint_err::<NoRedeclare>("function a() {} function a() {}", 25);
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = function() { }",
      28,
    );

    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = new Date();",
      28,
    );

    assert_lint_err_n::<NoRedeclare>(
      "var a = 3; var a = 10; var a = 15;",
      vec![15, 27],
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoRedeclare>("var a; var a;", 11);

    assert_lint_err::<NoRedeclare>("export var a; var a;", 18);
  }

  #[test]
  fn err_5() {
    assert_lint_err::<NoRedeclare>("var Object = 0;", 4);

    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12), (1, 22)],
    );
  }

  #[test]
  fn err_6() {
    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12), (1, 22)],
    );

    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12), (1, 22)],
    );

    assert_lint_err::<NoRedeclare>("var globalThis = 0;", 5);
  }

  #[test]
  fn err_7() {
    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: globalThis = 0} = {};",
      vec![(1, 12), (1, 22)],
    );
  }

  #[test]
  fn err_8() {
    assert_lint_err::<NoRedeclare>("function f() { var a; var a; }", 26);

    assert_lint_err::<NoRedeclare>("function f(a) { var a; }", 20);

    assert_lint_err::<NoRedeclare>(
      "function f() { var a; if (test) { var a; } }",
      38,
    );
  }

  #[test]
  fn err_9() {
    assert_lint_err::<NoRedeclare>("for (var a, a;;);", 12);

    assert_lint_err::<NoRedeclare>("let a; let a;", 11);

    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 13);
  }

  #[test]
  fn err_10() {
    assert_lint_err::<NoRedeclare>("var Object = 0;", 4);
  }

  #[test]
  fn err_11() {
    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 13);

    assert_lint_err::<NoRedeclare>("const a = 0; const a = 0;", 19);

    assert_lint_err::<NoRedeclare>("if (test) { let a; let a; }", 23);
  }

  #[test]
  fn err_12() {
    assert_lint_err::<NoRedeclare>(
      "switch (test) { case 0: let a; let a; }",
      35,
    );

    assert_lint_err::<NoRedeclare>("for (let a, a;;);", 12);

    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 13);
  }

  #[test]
  fn err_13() {
    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 13);

    assert_lint_err::<NoRedeclare>("function f() { let a; let a; }", 26);

    assert_lint_err::<NoRedeclare>("function f(a) { let a; }", 20);
  }

  #[test]
  fn err_14() {
    assert_lint_err::<NoRedeclare>(
      "function f() { if (test) { let a; let a; } }",
      38,
    );
  }
}
