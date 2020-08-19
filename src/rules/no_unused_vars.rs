// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::VarDeclarator;
use swc_ecmascript::utils::find_ids;
use swc_ecmascript::utils::ident::IdentLike;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

use std::collections::HashSet;
use std::sync::Arc;

pub struct NoUnusedVars;

impl LintRule for NoUnusedVars {
  fn new() -> Box<Self> {
    Box::new(NoUnusedVars)
  }

  fn code(&self) -> &'static str {
    "no-unused-vars"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = Collector {
      used_vars: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoUnusedVarVisitor::new(context, collector.used_vars);
    module.visit_with(module, &mut visitor);
  }
}

/// Collects information about variable usages.
struct Collector {
  used_vars: HashSet<Id>,
}

impl Visit for Collector {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc
  // It will make binary much smaller. In case of swc, binary size is reduced to 18mb from 29mb.
  // noop_visit_type!();

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    match expr {
      Expr::Ident(i) => {
        // Mark the variable as used.
        self.used_vars.insert(i.to_id());
      }
      _ => expr.visit_children_with(self),
    }
  }

  fn visit_pat(&mut self, pat: &Pat, _: &dyn Node) {
    match pat {
      // Ignore patterns
      Pat::Ident(..) | Pat::Invalid(..) => {}
      //
      _ => pat.visit_children_with(self),
    }
  }

  fn visit_member_expr(&mut self, member_expr: &MemberExpr, _: &dyn Node) {
    member_expr.obj.visit_with(member_expr, self);
    if member_expr.computed {
      member_expr.prop.visit_with(member_expr, self);
    }
  }
}

struct NoUnusedVarVisitor {
  context: Arc<Context>,
  used_vars: HashSet<Id>,
}

impl NoUnusedVarVisitor {
  fn new(context: Arc<Context>, used_vars: HashSet<Id>) -> Self {
    Self { context, used_vars }
  }
}

/// As we only care about variables, only variable declrations are checked.
impl Visit for NoUnusedVarVisitor {
  // TODO(kdy1): swc_ecmascript::visit::noop_visit_type!() after updating swc

  fn visit_var_declarator(&mut self, declarator: &VarDeclarator, _: &dyn Node) {
    let declared_idents: Vec<Ident> = find_ids(&declarator.name);

    for ident in declared_idents {
      if !self.used_vars.contains(&ident.to_id()) {
        // The variable is not used.
        self.context.add_diagnostic(
          ident.span,
          "no-unused-vars",
          &format!("\"{}\" label is never used", ident.sym),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unused_vars_ok() {
    assert_lint_ok::<NoUnusedVars>("var a = 1; console.log(a)");
    assert_lint_ok::<NoUnusedVars>(
      "var a = 1; function foo() { console.log(a) } ",
    );
    assert_lint_ok::<NoUnusedVars>(
      "var a = 1; const arrow = () => a; console.log(arrow)",
    );

    // Hoisting. This code is wrong, but it's not related with unused-vars
    assert_lint_ok::<NoUnusedVars>("console.log(a); var a = 1;");
  }

  #[test]
  fn no_unused_vars_err() {
    assert_lint_err::<NoUnusedVars>("var a = 0", 4);

    // variable shadowing
    assert_lint_err::<NoUnusedVars>(
      "var a = 1; function foo() { var a = 2; console.log(a); }",
      4,
    );
  }
}
