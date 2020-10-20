// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::Decl;
use swc_ecmascript::ast::Stmt;
use swc_ecmascript::ast::SwitchCase;
use swc_ecmascript::ast::VarDeclKind;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

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
    module.visit_all_with(module, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Requires lexical declarations (`let`, `const`, `function` and `class`) in
switch `case` or `default` clauses to be scoped with brackets.

Without brackets in the `case` or `default` block, the lexical declarations are
visible to the entire switch block but only get initialized when they are assigned,
which only happens if that case/default is reached.  This can lead to unexpected
errors.  The solution is to ensure each `case` or `default` block is wrapped in
brackets to scope limit the declarations.

### Valid:
```typescript
switch (choice) {
  // The following `case` and `default` clauses are wrapped into blocks using brackets
  case 1: {
      let a = "choice 1";
      break;
  }
  case 2: {
      const b = "choice 2";
      break;
  }
  case 3: {
      function f() { return "choice 3"; }
      break;
  }
  default: {
      class C {}
  }
}
```

### Invalid:
```typescript
switch (choice) {
  // `let`, `const`, `function` and `class` are scoped the entire switch statement here
  case 1:
      let a = "choice 1";
      break;
  case 2:
      const b = "choice 2";
      break;
  case 3:
      function f() { return "choice 3"; }
      break;
  default:
      class C {}
}
```"#
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

impl<'c> VisitAll for NoCaseDeclarationsVisitor<'c> {
  noop_visit_type!();

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
        self.context.add_diagnostic_with_hint(
          switch_case.span,
          "no-case-declarations",
          "Unexpected declaration in case",
          "Wrap switch case and default blocks in brackets",
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
    assert_lint_ok! {
      NoCaseDeclarations,
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
    };
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
