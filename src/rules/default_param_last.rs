// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{Function, Pat};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct DefaultParamLast;

impl LintRule for DefaultParamLast {
  fn new() -> Box<Self> {
    Box::new(DefaultParamLast)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = DefaultParamLastVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct DefaultParamLastVisitor {
  context: Context,
}

impl DefaultParamLastVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for DefaultParamLastVisitor {
  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    let pat = function.params.iter().rev().find_map(|param| match param {
      Pat::Assign(pat) => Some(pat),
      _ => None,
    });
    if let Some(pat) = pat {
      self.context.add_diagnostic(
        pat.span,
        "defaultParamLast",
        "default parameters should be at last",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn default_param_last_test() {
    test_lint(
      "default_param_last",
      "function fn(a = 2, b) {}",
      vec![DefaultParamLast::new()],
      json!([{
        "code": "defaultParamLast",
        "message": "default parameters should be at last",
        "location": {
          "filename": "default_param_last",
          "line": 1,
          "col": 12
        }
      }]),
    );
  }
}
