// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::{
  ast::*, utils::find_ids, utils::ident::IdentLike, utils::Id, visit::Node,
  visit::Visit,
};
use swc_ecmascript::{
  ast::{Ident, Module},
  visit::{noop_visit_type, VisitWith},
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
      context: context,
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
    if !self.bindings.insert(i.to_id()) {
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
    self.declare(&f.ident)
  }

  fn visit_var_decl(&mut self, v: &VarDecl, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&v.decls);

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
  #[ignore = "List of globals will be added by #304"]
  fn err_5() {
    assert_lint_err::<NoRedeclare>("var Object = 0;", 0);

    assert_lint_err::<NoRedeclare>("var top = 0;", 0);

    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(0, 0), (0, 0)],
    );
  }

  #[test]
  #[ignore = "List of globals will be added by #304"]
  fn err_6() {
    assert_lint_err::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      0,
    );

    assert_lint_err::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      0,
    );

    assert_lint_err::<NoRedeclare>("var globalThis = 0;", 0);
  }

  #[test]
  #[ignore = "List of globals will be added by #304"]
  fn err_7() {
    assert_lint_err::<NoRedeclare>(
      "var a; var {a = 0, b: globalThis = 0} = {};",
      0,
    );
  }

  #[test]
  fn err_8() {
    assert_lint_err::<NoRedeclare>("function f() { var a; var a; }", 0);

    assert_lint_err::<NoRedeclare>("function f(a) { var a; }", 0);

    assert_lint_err::<NoRedeclare>(
      "function f() { var a; if (test) { var a; } }",
      0,
    );
  }

  #[test]
  fn err_9() {
    assert_lint_err::<NoRedeclare>("for (var a, a;;);", 12);

    assert_lint_err::<NoRedeclare>("let a; let a;", 11);

    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 13);
  }

  #[test]
  #[ignore = "List of globals will be added by #304"]
  fn err_10() {
    assert_lint_err::<NoRedeclare>("var Object = 0;", 0);
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

    assert_lint_err::<NoRedeclare>("function f() { let a; let a; }", 0);

    assert_lint_err::<NoRedeclare>("function f(a) { let a; }", 0);
  }

  #[test]
  fn err_14() {
    assert_lint_err::<NoRedeclare>(
      "function f() { if (test) { let a; let a; } }",
      0,
    );
  }
}
