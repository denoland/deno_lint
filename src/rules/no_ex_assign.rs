// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::scopes::Scope;
use swc_common;
use swc_ecmascript::ast::{AssignExpr, ObjectPatProp, Pat, PatOrExpr};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoExAssign;

impl LintRule for NoExAssign {
  fn new() -> Box<Self> {
    Box::new(NoExAssign)
  }

  fn code(&self) -> &'static str {
    "no-ex-assign"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecmascript::ast::Module) {
    let mut visitor = NoExAssignVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoExAssignVisitor {
  context: Arc<Context>,
}

impl NoExAssignVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn check_scope_for_catch_clause(
    &self,
    scope: &Scope,
    name: impl AsRef<str>,
    span: swc_common::Span,
  ) {
    if let Some(BindingKind::CatchClause) = scope.get_binding(name) {
      self.context.add_diagnostic(
        span,
        "no-ex-assign",
        "Reassigning exception parameter is not allowed",
      );
    }
  }
}

impl Visit for NoExAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    let scope = self.context.root_scope.get_scope_for_span(assign_expr.span);
    match &assign_expr.left {
      PatOrExpr::Expr(_) => {}
      PatOrExpr::Pat(boxed_pat) => match &**boxed_pat {
        Pat::Ident(ident) => self.check_scope_for_catch_clause(
          &scope,
          &ident.sym,
          assign_expr.span,
        ),
        Pat::Array(array) => {
          if array.elems.is_empty() {
            return;
          }
          for elem in array.elems.iter() {
            if let Some(Pat::Ident(ident)) = elem {
              self.check_scope_for_catch_clause(
                &scope,
                &ident.sym,
                assign_expr.span,
              );
            }
          }
        }
        Pat::Object(object) => {
          if object.props.is_empty() {
            return;
          }
          for prop in object.props.iter() {
            if let ObjectPatProp::KeyValue(kv) = prop {
              if let Pat::Assign(assign_pat) = &*kv.value {
                if let Pat::Ident(ident) = &*assign_pat.left {
                  self.check_scope_for_catch_clause(
                    &scope,
                    &ident.sym,
                    assign_expr.span,
                  );
                }
              }
            }
          }
        }
        _ => {}
      },
    };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_err_on_line_n;
  use crate::test_util::assert_lint_ok;

  #[test]
  fn no_ex_assign_valid() {
    assert_lint_ok::<NoExAssign>(
      r#"
try {} catch { e = 1; }
try {} catch (ex) { something = 1; }
try {} catch (ex) { return 1; }
function foo() { try { } catch (e) { return false; } }
      "#,
    );
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
