// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Decl;
use swc_ecma_ast::Stmt;
use swc_ecma_ast::SwitchCase;
use swc_ecma_ast::VarDeclKind;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoCaseDeclarations;

impl LintRule for NoCaseDeclarations {
  fn new() -> Box<Self> {
    Box::new(NoCaseDeclarations)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoCaseDeclarationsVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoCaseDeclarationsVisitor {
  context: Context,
}

impl NoCaseDeclarationsVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoCaseDeclarationsVisitor {
  fn visit_switch_case(
    &mut self,
    switch_case: &SwitchCase,
    _parent: &dyn Node,
  ) {
    for stmt in &switch_case.cons {
      let is_lexical_decl = match stmt {
        Stmt::Decl(decl) => match &decl {
          Decl::Fn(_) => true,
          Decl::Class(_) => true,
          Decl::Var(var_decl) => var_decl.kind != VarDeclKind::Var,
          _ => false,
        },
        _ => false,
      };

      if is_lexical_decl {
        self.context.add_diagnostic(
          switch_case.span,
          "noCaseDeclarations",
          "Unexpected declaration in case",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn no_case_declarations_ok() {
    test_lint(
      "no_with",
      r#"
switch (foo) {
  case 1: {
    let a = "a";
    break;
  }
  case 2: {
    const a = "a";
    break;
  }
  case 3: {
    function foobar() {

    }
    break;
  }
  case 4: {
    class Foobar {
      
    }
    break;
  }
  default: {
    let b = "b";
    break;
  }
}
      "#,
      vec![NoCaseDeclarations::new()],
      json!([]),
    )
  }

  #[test]
  fn no_case_declarations() {
    test_lint(
      "no_case_declarations",
      r#"
switch (foo) {
  case 1:
    let a = "a";
    break;
}

switch (bar) {
  default:
    let a = "a";
    break;
}

switch (fizz) {
  case 1:
    const a = "a";
    break;
}

switch (buzz) {
  default:
    const a = "a";
    break;
}

switch (fncase) {
  case 1:
    function fn() {

    }
    break;
}

switch (classcase) {
  case 1:
    class Cl {
      
    }
    break;
}
      "#,
      vec![NoCaseDeclarations::new()],
      json!([{
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 3,
          "col": 2,
        }
      },
      {
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 9,
          "col": 2,
        }
      },
      {
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 15,
          "col": 2,
        }
      },
      {
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 21,
          "col": 2,
        }
      },
      {
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 27,
          "col": 2,
        }
      },
      {
        "code": "noCaseDeclarations",
        "message": "Unexpected declaration in case",
        "location": {
          "filename": "no_case_declarations",
          "line": 35,
          "col": 2,
        }
      }]),
    )
  }
}
