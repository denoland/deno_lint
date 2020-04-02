// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::HashSet;
use swc_common::Span;
use swc_ecma_ast::ArrowExpr;
use swc_ecma_ast::Function;
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

  fn check_params(&self, span: Span, params: &[Pat]) {
    let mut seen: HashSet<String> = HashSet::new();

    for param in params {
      match &param {
        Pat::Ident(ident) => {
          let param_name = ident.sym.to_string();

          if seen.get(&param_name).is_some() {
            self.context.add_diagnostic(
              span,
              "noDupeArgs",
              "Duplicate arguments not allowed",
            );
          } else {
            seen.insert(param_name);
          }
        }
        _ => continue,
      }
    }
  }
}

impl Visit for NoDupeArgsVisitor {
  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(function.span, &function.params);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_params(arrow_expr.span, &arrow_expr.params);
  }
}
