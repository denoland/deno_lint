// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Decl;
use swc_ecmascript::ast::Stmt;
use swc_ecmascript::ast::SwitchCase;
use swc_ecmascript::ast::VarDeclKind;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct NoCaseDeclarations;

impl LintRule for NoCaseDeclarations {
  fn new() -> Box<Self> {
    Box::new(NoCaseDeclarations)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-case-declarations"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoCaseDeclarationsVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoCaseDeclarationsVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoCaseDeclarationsVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoCaseDeclarationsVisitor<'c> {
  noop_visit_type!();

  fn visit_switch_case(
    &mut self,
    switch_case: &SwitchCase,
    _parent: &dyn Node,
  ) {
    switch_case.visit_children_with(self);

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
          "no-case-declarations",
          "Unexpected declaration in case",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_case_declarations_valid() {
    assert_lint_ok::<NoCaseDeclarations>(
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
    );
  }

  #[test]
  fn no_case_declarations_invalid() {
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (foo) {
  case 1:
    let a = "a";
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (bar) {
  default:
    let a = "a";
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (fizz) {
  case 1:
    const a = "a";
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (buzz) {
  default:
    const a = "a";
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (fncase) {
  case 1:
    function fn() {

    }
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (fncase) {
  default:
    function fn() {

    }
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (classcase) {
  case 1:
    class Cl {
      
    }
    break;
}
    "#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (classcase) {
  default:
    class Cl {
      
    }
    break;
}
    "#,
      3,
      2,
    );

    // nested switch
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (foo) {
  case 1:
    switch (bar) {
      case 2:
        let a = "a";
        break;
    }
    break;
}
    "#,
      5,
      6,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (foo) {
  default:
    switch (bar) {
      default:
        const a = "a";
        break;
    }
    break;
}
    "#,
      5,
      6,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (foo) {
  case 1:
    switch (bar) {
      default:
        function fn() {}
        break;
    }
    break;
}
    "#,
      5,
      6,
    );
    assert_lint_err_on_line::<NoCaseDeclarations>(
      r#"
switch (foo) {
  default:
    switch (bar) {
      case 1:
        class Cl {}
        break;
    }
    break;
}
    "#,
      5,
      6,
    );
  }
}
