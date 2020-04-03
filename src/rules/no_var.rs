// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::VarDecl;
use swc_ecma_ast::VarDeclKind;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoVar;

impl LintRule for NoVar {
  fn new() -> Box<Self> {
    Box::new(NoVar)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoVarVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoVarVisitor {
  context: Context,
}

impl NoVarVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoVarVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.kind == VarDeclKind::Var {
      self.context.add_diagnostic(
        var_decl.span,
        "noVar",
        "`var` keyword is not allowed",
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
  fn no_var_test() {
    test_lint(
      "no_var",
      r#"
var someVar = "someString";
const c = "c";
let a = "a";
      "#,
      vec![NoVar::new()],
      json!([{
        "code": "noVar",
        "message": "`var` keyword is not allowed",
        "location": {
          "filename": "no_var",
          "line": 2,
          "col": 0,
        }
      }]),
    )
  }
}
