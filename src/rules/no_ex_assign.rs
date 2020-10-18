// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::{scopes::BindingKind, swc_util::find_lhs_ids};

use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoExAssign;

impl LintRule for NoExAssign {
  fn new() -> Box<Self> {
    Box::new(NoExAssign)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-ex-assign"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoExAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoExAssignVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoExAssignVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoExAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let ids = find_lhs_ids(&assign_expr.left);

    for id in ids {
      let var = self.context.scope.var(&id);

      if let Some(var) = var {
        if let BindingKind::CatchClause = var.kind() {
          self.context.add_diagnostic(
            assign_expr.span,
            "no-ex-assign",
            "Reassigning exception parameter is not allowed",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line_n;

  #[test]
  fn no_ex_assign_valid() {
    assert_lint_ok! {
      NoExAssign,
      r#"
try {} catch { e = 1; }
try {} catch (ex) { something = 1; }
try {} catch (ex) { return 1; }
function foo() { try { } catch (e) { return false; } }
      "#,
    };
  }

  #[test]
  fn no_ex_assign_invalid() {
    assert_lint_err_on_line_n::<NoExAssign>(
      r#"
try {} catch (e) { e = 1; }
try {} catch (ex) { ex = 1; }
try {} catch (ex) { [ex] = []; }
try {} catch (ex) { ({x: ex = 0} = {}); }
try {} catch ({message}) { message = 1; }
      "#,
      vec![(2, 19), (3, 20), (4, 20), (5, 21), (6, 27)],
    );
  }
}
