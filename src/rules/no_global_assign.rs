use super::LintRule;
use crate::linter::Context;
use std::{collections::HashSet, sync::Arc};
use swc_common::SyntaxContext;
use swc_ecmascript::{
  ast::AssignExpr,
  ast::FnDecl,
  utils::Id,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};

pub struct NoGlobalAssign;

impl LintRule for NoGlobalAssign {
  fn new() -> Box<Self> {
    Box::new(NoGlobalAssign)
  }

  fn code(&self) -> &'static str {
    "no-global-assign"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = TopLevelBindingCollector {
      bindings: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoGlobalAssignVisitor::new(context, collector.bindings);
    module.visit_with(module, &mut visitor);
  }
}

struct TopLevelBindingCollector {
  bindings: HashSet<Id>,
}

impl Visit for TopLevelBindingCollector {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, i: &FnDecl, _: &dyn Node) {}
}

struct NoGlobalAssignVisitor {
  context: Arc<Context>,
  /// This hashset only contains top level bindings, so using HashSet<JsWord>
  /// also can be an option.
  bindings: HashSet<Id>,
}

impl NoGlobalAssignVisitor {
  fn new(context: Arc<Context>, bindings: HashSet<Id>) -> Self {
    Self { context, bindings }
  }
}

impl Visit for NoGlobalAssignVisitor {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, e: &AssignExpr, _: &dyn Node) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line_n;
  use crate::test_util::assert_lint_ok;
}
