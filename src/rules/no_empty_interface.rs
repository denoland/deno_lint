// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsInterfaceDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyInterface;

impl LintRule for NoEmptyInterface {
  fn new() -> Box<Self> {
    Box::new(NoEmptyInterface)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEmptyInterfaceVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoEmptyInterfaceVisitor {
  context: Context,
}

impl NoEmptyInterfaceVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyInterfaceVisitor {
  fn visit_ts_interface_decl(
    &mut self,
    interface_decl: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    if interface_decl.body.body.is_empty() {
      self.context.add_diagnostic(
        interface_decl.span,
        "noEmptyInterface",
        "Empty interfaces are not allowed",
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
  fn no_empty_interface() {
    test_lint(
      "no_empty_interface",
      r#"
interface EmptyInterface {}

interface NonEmptyInterface {
  a: string
}
      "#,
      vec![NoEmptyInterface::new()],
      json!([{
        "code": "noEmptyInterface",
        "message": "Empty interfaces are not allowed",
        "location": {
          "filename": "no_empty_interface",
          "line": 2,
          "col": 0,
        }
      }]),
    )
  }
}
