// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::ArrowExpr;
use crate::swc_ecma_ast::Function;
use crate::swc_ecma_ast::Param;
use crate::swc_ecma_ast::Pat;
use std::collections::HashSet;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct NoDupeArgs;

impl LintRule for NoDupeArgs {
  fn new() -> Box<Self> {
    Box::new(NoDupeArgs)
  }

  fn code(&self) -> &'static str {
    "no-dupe-args"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoDupeArgsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDupeArgsVisitor {
  context: Arc<Context>,
}

impl NoDupeArgsVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn check_pats(&self, span: Span, pats: &[Pat]) {
    let mut seen: HashSet<String> = HashSet::new();

    for pat in pats {
      match &pat {
        Pat::Ident(ident) => {
          if !seen.insert(ident.sym.to_string()) {
            self.context.add_diagnostic(
              span,
              "no-dupe-args",
              "Duplicate arguments not allowed",
            );
          }
        }
        _ => continue,
      }
    }
  }

  fn check_params(&self, span: Span, params: &[Param]) {
    let pats = params
      .iter()
      .map(|param| param.pat.clone())
      .collect::<Vec<Pat>>();
    self.check_pats(span, &pats);
  }
}

impl Visit for NoDupeArgsVisitor {
  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(function.span, &function.params);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_pats(arrow_expr.span, &arrow_expr.params);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_dupe_args_test() {
    assert_lint_err::<NoDupeArgs>("function dupeArgs1(a, b, a) { }", 0);
    assert_lint_err::<NoDupeArgs>("const dupeArgs2 = (a, b, a) => { }", 18);
  }
}
