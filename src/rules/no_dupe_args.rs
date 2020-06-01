// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::HashSet;
use swc_common::Span;
use swc_ecma_ast::ArrowExpr;
use swc_ecma_ast::Function;
use swc_ecma_ast::Param;
use swc_ecma_ast::Pat;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDupeArgs;

impl LintRule for NoDupeArgs {
  fn new() -> Box<Self> {
    Box::new(NoDupeArgs)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoDupeArgsVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoDupeArgsVisitor {
  context: Context,
}

impl NoDupeArgsVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_pats(&self, span: Span, pats: &[Pat]) {
    let mut seen: HashSet<String> = HashSet::new();

    for pat in pats {
      match &pat {
        Pat::Ident(ident) => {
          let pat_name = ident.sym.to_string();

          if seen.get(&pat_name).is_some() {
            self.context.add_diagnostic(
              span,
              "noDupeArgs",
              "Duplicate arguments not allowed",
            );
          } else {
            seen.insert(pat_name);
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
    assert_lint_err::<NoDupeArgs>(
      "function dupeArgs1(a, b, a) { }",
      "noDupeArgs",
      0,
    );
    assert_lint_err::<NoDupeArgs>(
      "const dupeArgs2 = (a, b, a) => { }",
      "noDupeArgs",
      18,
    );
  }
}
