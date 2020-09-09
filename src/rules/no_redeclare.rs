// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_atoms::JsWord;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::{
  ast::{Ident, Module},
  visit::{noop_visit_type, Visit, VisitWith},
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
    let mut collector = Collector {
      decalred_vars: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoRedeclareVisitor {
      context: context,
      decalred_vars: collector.decalred_vars,
    };
    module.visit_with(module, &mut visitor);
  }
}

/// Collects information about variable usages.
struct Collector {
  // TODO(kdy1): Change this to HashMap<JsWord, Span> and point previous declaration
  decalred_vars: HashSet<JsWord>,
}

impl Collector {
  fn decare(&mut self, i: &Ident) {
    self.decalred_vars.insert(i.sym.clone());
  }
}

impl Visit for Collector {}

struct NoRedeclareVisitor {
  context: Arc<Context>,
  decalred_vars: HashSet<JsWord>,
}

impl Visit for NoRedeclareVisitor {
  noop_visit_type!();
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
    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 0);

    assert_lint_err::<NoRedeclare>(
      "switch(foo) { case a: var b = 3;\ncase b: var b = 4}",
      0,
    );

    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 0);
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoRedeclare>("var a = {}; var a = [];", 0);

    assert_lint_err::<NoRedeclare>("var a; function a() {}", 0);

    assert_lint_err::<NoRedeclare>("function a() {} function a() {}", 0);
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = function() { }",
      0,
    );

    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = new Date();",
      0,
    );

    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10; var a = 15;", 0);
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoRedeclare>("var a; var a;", 0);

    assert_lint_err::<NoRedeclare>("export var a; var a;", 0);
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
    assert_lint_err::<NoRedeclare>("for (var a, a;;);", 0);

    assert_lint_err::<NoRedeclare>("let a; let a;", 0);

    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 0);
  }

  #[test]
  #[ignore = "List of globals will be added by #304"]
  fn err_10() {
    assert_lint_err::<NoRedeclare>("var Object = 0;", 0);
  }

  #[test]
  fn err_11() {
    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 0);

    assert_lint_err::<NoRedeclare>("const a = 0; const a = 0;", 0);

    assert_lint_err::<NoRedeclare>("if (test) { let a; let a; }", 0);
  }

  #[test]
  fn err_12() {
    assert_lint_err::<NoRedeclare>(
      "switch (test) { case 0: let a; let a; }",
      0,
    );

    assert_lint_err::<NoRedeclare>("for (let a, a;;);", 0);

    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 0);
  }

  #[test]
  fn err_13() {
    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 0);

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
