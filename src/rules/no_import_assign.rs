use super::LintRule;
use crate::linter::Context;
use std::{collections::HashSet, sync::Arc};
use swc_ecmascript::{
  ast::*,
  utils::ident::IdentLike,
  utils::Id,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};

pub struct NoImportAssign;

impl LintRule for NoImportAssign {
  fn new() -> Box<Self> {
    Box::new(NoImportAssign)
  }

  fn code(&self) -> &'static str {
    "no-import-assign"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = Collector {
      imports: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoImportAssignVisitor::new(context, collector.imports);
    module.visit_with(module, &mut visitor);
  }
}

struct Collector {
  imports: HashSet<Id>,
}

impl Visit for Collector {
  noop_visit_type!();

  fn visit_import_named_specifier(
    &mut self,
    i: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.imports.insert(i.local.to_id());
  }

  fn visit_import_default_specifier(
    &mut self,
    i: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.imports.insert(i.local.to_id());
  }

  fn visit_import_star_as_specifier(
    &mut self,
    i: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.imports.insert(i.local.to_id());
  }
}

struct NoImportAssignVisitor {
  context: Arc<Context>,
  /// This hashset only contains top level bindings, so using HashSet<JsWord>
  /// also can be an option.
  imports: HashSet<Id>,
}

impl NoImportAssignVisitor {
  fn new(context: Arc<Context>, imports: HashSet<Id>) -> Self {
    Self { context, imports }
  }

  fn check(&self, i: Id) {
    if self.imports.contains(&i) {}
  }
}

impl Visit for NoImportAssignVisitor {
  noop_visit_type!();

  fn visit_pat(&mut self, n: &Pat, _: &dyn Node) {
    if let Pat::Ident(i) = n {
      self.check(i.to_id());
    } else {
      n.visit_children_with(self);
    }
  }

  fn visit_assign_pat_prop(&mut self, n: &AssignPatProp, _: &dyn Node) {
    self.check(n.key.to_id());

    n.value.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }
}
