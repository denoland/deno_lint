// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::VarDecl;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct SingleVarDeclarator;

impl LintRule for SingleVarDeclarator {
  fn new() -> Box<Self> {
    Box::new(SingleVarDeclarator)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = SingleVarDeclaratorVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct SingleVarDeclaratorVisitor {
  context: Context,
}

impl SingleVarDeclaratorVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for SingleVarDeclaratorVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if var_decl.decls.len() > 1 {
      self.context.add_diagnostic(
        var_decl.span,
        "singleVarDeclarator",
        "Multiple variable declarators are not allowed",
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
  fn single_var_declarator_test() {
    test_lint(
      "single_var_declarator",
      r#"
const a1 = "a", b1 = "b", c1 = "c";

let a2 = "a", b2 = "b", c2 = "c";

var a3 = "a", b3 = "b", c3 = "c";
      "#,
      vec![SingleVarDeclarator::new()],
      json!([{
        "code": "singleVarDeclarator",
        "message": "Multiple variable declarators are not allowed",
        "location": {
          "filename": "single_var_declarator",
          "line": 2,
          "col": 0,
        }
      }, {
        "code": "singleVarDeclarator",
        "message": "Multiple variable declarators are not allowed",
        "location": {
          "filename": "single_var_declarator",
          "line": 4,
          "col": 0,
        }
      }, {
        "code": "singleVarDeclarator",
        "message": "Multiple variable declarators are not allowed",
        "location": {
          "filename": "single_var_declarator",
          "line": 6,
          "col": 0,
        }
      }]),
    )
  }
}
