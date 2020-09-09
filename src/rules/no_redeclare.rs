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

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_11() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_12() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_13() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_14() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }

  #[test]
  fn ok_15() {
    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");

    assert_lint_ok::<NoRedeclare>("");
  }
}
