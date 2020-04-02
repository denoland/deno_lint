// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use std::collections::HashSet;
use swc_common::Span;
use swc_ecma_ast::ArrowExpr;
use swc_ecma_ast::Function;
use swc_ecma_ast::Pat;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoDupeArgs {
  context: Context,
}

impl NoDupeArgs {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn check_params(&self, span: &Span, params: &[Pat]) {
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

impl Visit for NoDupeArgs {
  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    self.check_params(&function.span, &function.params);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.check_params(&arrow_expr.span, &arrow_expr.params);
  }
}
