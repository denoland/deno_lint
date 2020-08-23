// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{
  ArrowExpr, AssignExpr, CatchClause, Expr, FnDecl, FnExpr, Ident, Module,
  ObjectPatProp, Pat, PatOrExpr, VarDecl,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

use std::sync::Arc;

pub struct NoShadowRestrictedNames;

impl LintRule for NoShadowRestrictedNames {
  fn new() -> Box<Self> {
    Box::new(NoShadowRestrictedNames)
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoShadowRestrictedNamesVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn code(&self) -> &'static str {
    "no-shadow-restricted-names"
  }
}

struct NoShadowRestrictedNamesVisitor {
  context: Arc<Context>,
}

impl NoShadowRestrictedNamesVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn is_restricted_names(&self, ident: &Ident) -> bool {
    match ident.sym.as_ref() {
      "undefined" | "NaN" | "Infinity" | "arguments" | "eval" => true,
      _ => false,
    }
  }

  fn check_pat(&self, pat: &Pat, check_scope: bool) {
    match pat {
      Pat::Ident(ident) => {
        // trying to assign `undefined`
        // Check is scope is valid for current pattern
        if &ident.sym == "undefined" && check_scope {
          let scope = self.context.root_scope.get_scope_for_span(ident.span);
          if let Some(_binding) = scope.get_binding(&ident.sym) {
            self.report_shadowing(&ident);
          }
          return;
        }

        self.check_shadowing(ident);
      }
      Pat::Expr(expr) => {
        if let Expr::Ident(ident) = expr.as_ref() {
          self.check_shadowing(ident);
        }
      }
      Pat::Array(array_pat) => {
        for el in &array_pat.elems {
          if el.is_some() {
            self.check_pat(el.as_ref().unwrap(), false);
          }
        }
      }
      Pat::Object(object_pat) => {
        for prop in &object_pat.props {
          match prop {
            ObjectPatProp::Assign(assign) => {
              self.check_shadowing(&assign.key);
            }
            ObjectPatProp::Rest(rest) => self.check_pat(&rest.arg, false),
            ObjectPatProp::KeyValue(key_value) => {
              self.check_pat(&key_value.value, false);
            }
          }
        }
      }
      Pat::Rest(rest_pat) => {
        self.check_pat(&rest_pat.arg, false);
      }
      _ => {}
    }
  }

  fn check_shadowing(&self, ident: &Ident) {
    if self.is_restricted_names(&ident) {
      self.report_shadowing(&ident);
    }
  }

  fn report_shadowing(&self, ident: &Ident) {
    self.context.add_diagnostic(
      ident.span,
      "no-shadow-restricted-names",
      &format!("Shadowing of global property {}", &ident.sym),
    );
  }
}

impl Visit for NoShadowRestrictedNamesVisitor {
  noop_visit_type!();

  fn visit_var_decl(&mut self, node: &VarDecl, parent: &dyn Node) {
    for decl in &node.decls {
      if let Pat::Ident(ident) = &decl.name {
        // `undefined` variable declaration without init is have same meaning
        if decl.init.is_none() && &ident.sym == "undefined" {
          continue;
        }
      }

      self.check_pat(&decl.name, false);
    }

    swc_ecmascript::visit::visit_var_decl(self, node, parent);
  }

  fn visit_fn_decl(&mut self, node: &FnDecl, parent: &dyn Node) {
    self.check_shadowing(&node.ident);

    for param in &node.function.params {
      self.check_pat(&param.pat, false);
    }

    swc_ecmascript::visit::visit_fn_decl(self, node, parent);
  }

  fn visit_fn_expr(&mut self, node: &FnExpr, parent: &dyn Node) {
    if node.ident.is_some() {
      self.check_shadowing(node.ident.as_ref().unwrap())
    }

    for param in &node.function.params {
      self.check_pat(&param.pat, false);
    }

    swc_ecmascript::visit::visit_fn_expr(self, node, parent);
  }

  fn visit_arrow_expr(&mut self, node: &ArrowExpr, parent: &dyn Node) {
    for param in &node.params {
      self.check_pat(&param, false);
    }

    swc_ecmascript::visit::visit_arrow_expr(self, node, parent);
  }

  fn visit_catch_clause(&mut self, node: &CatchClause, parent: &dyn Node) {
    if node.param.is_some() {
      self.check_pat(node.param.as_ref().unwrap(), false);
    }

    swc_ecmascript::visit::visit_catch_clause(self, node, parent);
  }

  fn visit_assign_expr(&mut self, node: &AssignExpr, _parent: &dyn Node) {
    if let PatOrExpr::Pat(pat) = &node.left {
      self.check_pat(pat, true);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_shadow_restricted_names_valid() {
    assert_lint_ok::<NoShadowRestrictedNames>("function foo(bar){ var baz; }");
    assert_lint_ok::<NoShadowRestrictedNames>("!function foo(bar){ var baz; }");
    assert_lint_ok::<NoShadowRestrictedNames>("!function(bar){ var baz; }");
    assert_lint_ok::<NoShadowRestrictedNames>("try {} catch(e) {}");
    assert_lint_ok::<NoShadowRestrictedNames>("export default function() {}");
    assert_lint_ok::<NoShadowRestrictedNames>("try {} catch {}");
    assert_lint_ok::<NoShadowRestrictedNames>("var undefined;");
    assert_lint_ok::<NoShadowRestrictedNames>(
      "var undefined; doSomething(undefined);",
    );
    assert_lint_ok::<NoShadowRestrictedNames>("var undefined; var undefined;");
    assert_lint_ok::<NoShadowRestrictedNames>("let undefined");
    assert_lint_ok::<NoShadowRestrictedNames>("let [...foo] = []");
    assert_lint_ok::<NoShadowRestrictedNames>("function bar (...rest) {}");
  }

  #[test]
  fn no_shadow_restricted_names_invalid() {
    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "function NaN(NaN) { var NaN; !function NaN(NaN) { try {} catch(NaN) {} }; }",
      vec![
        (1, 9),
        (1, 13),
        (1, 24),
        (1, 39),
        (1, 43),
        (1, 63),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "function undefined(undefined) { !function undefined(undefined) { try {} catch(undefined) {} }; }",
      vec![
        (1, 9),
        (1, 19),
        (1, 42),
        (1, 52),
        (1, 78),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "function Infinity(Infinity) { var Infinity; !function Infinity(Infinity) { try {} catch(Infinity) {} }; }",
      vec![
        (1, 9),
        (1, 18),
        (1, 34),
        (1, 54),
        (1, 63),
        (1, 88),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "function arguments(arguments) { var arguments; !function arguments(arguments) { try {} catch(arguments) {} }; }",
      vec![
        (1, 9),
        (1, 19),
        (1, 36),
        (1, 57),
        (1, 67),
        (1, 93),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "function eval(eval) { var eval; !function eval(eval) { try {} catch(eval) {} }; }",
      vec![
        (1, 9),
        (1, 14),
        (1, 26),
        (1, 42),
        (1, 47),
        (1, 68),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "var eval = (eval) => { var eval; !function eval(eval) { try {} catch(eval) {} }; }",
      vec![
        (1, 4),
        (1, 12),
        (1, 27),
        (1, 43),
        (1, 48),
        (1, 69),
      ]
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      "var {undefined} = obj; var {a: undefined} = obj; var {a: {b: {undefined}}} = obj; var {a, ...undefined} = obj;",
      vec![
        (1, 5),
        (1, 31),
        (1, 62),
        (1, 93),
      ]
    );

    assert_lint_err::<NoShadowRestrictedNames>("var [undefined] = [1]", 5);
    assert_lint_err::<NoShadowRestrictedNames>(
      "var undefined; undefined = 5;",
      15,
    );
    assert_lint_err::<NoShadowRestrictedNames>("var [...undefined] = []", 8);
    assert_lint_err::<NoShadowRestrictedNames>(
      "try {} catch { try{} catch(NaN) {} }",
      27,
    );

    assert_lint_err_on_line_n::<NoShadowRestrictedNames>(
      r#"
function foo1(...undefined) {}
function foo2(...NaN) {}
function foo3(...arguments) {}
function foo4(...Infinity) {}
function foo5(...eval) {}
      "#,
      vec![(2, 17), (3, 17), (4, 17), (5, 17), (6, 17)],
    )
  }
}
