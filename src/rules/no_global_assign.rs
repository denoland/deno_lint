// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use swc_common::SyntaxContext;
use swc_ecmascript::{
  ast::*,
  utils::ident::IdentLike,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};
use swc_ecmascript::{utils::find_ids, utils::Id};

use crate::swc_util::find_lhs_ids;

use super::Context;
use super::LintRule;

use std::collections::HashSet;
use std::sync::Arc;

pub struct NoGlobalAssign;

impl LintRule for NoGlobalAssign {
  fn new() -> Box<Self> {
    Box::new(NoGlobalAssign)
  }

  fn code(&self) -> &'static str {
    "no-global-assign"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut collector = TopLevelBindingCollector {
      top_level_ctxt: context.top_level_ctxt,
      declared: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoGlobalAssignVisitor::new(context, collector.declared);
    module.visit_with(module, &mut visitor);
  }
}

/// Collects top level bindings, which have top level syntax context passed to the resolver.
struct TopLevelBindingCollector {
  /// The syntax context of the top level binding.
  ///
  /// Top level bindings and unresolved reference identifiers are marked with this.
  top_level_ctxt: SyntaxContext,
  /// If there exists a binding with such id, it's not global.
  declared: HashSet<Id>,
}

impl TopLevelBindingCollector {
  fn declare(&mut self, i: Id) {
    // Optimization
    if i.1 != self.top_level_ctxt {
      return;
    }
    self.declared.insert(i);
  }
}

impl Visit for TopLevelBindingCollector {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, f: &FnDecl, _: &dyn Node) {
    self.declare(f.ident.to_id());
  }
  fn visit_class_decl(&mut self, f: &ClassDecl, _: &dyn Node) {
    self.declare(f.ident.to_id());
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
}

struct NoGlobalAssignVisitor {
  context: Arc<Context>,
  declared: HashSet<Id>,
}

impl NoGlobalAssignVisitor {
  fn new(context: Arc<Context>, declared: HashSet<Id>) -> Self {
    Self { context, declared }
  }
}

impl Visit for NoGlobalAssignVisitor {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, e: &AssignExpr, _: &dyn Node) {
    e.visit_children_with(self);

    // We need span
    let idents: Vec<Ident> = find_lhs_ids(&e.left);

    for ident in idents {
      // We don't care about local references
      if ident.span.ctxt != self.context.top_level_ctxt {
        continue;
      }

      // Ignore top level bindings declared in the file.
      if self.declared.contains(&ident.to_id()) {
        continue;
      }

      self.context.add_diagnostic(
        ident.span,
        "no-global-assign",
        "Assigning to global is not allowed",
      )
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoGlobalAssign>(
      r#"
      string = 'hello world';",
      var string;
    "#,
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err("", col)
  }
}
