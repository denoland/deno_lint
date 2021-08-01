// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::Decl;
use swc_ecmascript::ast::Stmt;
use swc_ecmascript::ast::SwitchCase;
use swc_ecmascript::ast::VarDeclKind;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct NoCaseDeclarations;

const CODE: &str = "no-case-declarations";
const MESSAGE: &str = "Unexpected declaration in case";
const HINT: &str = "Wrap switch case and default blocks in brackets";

impl LintRule for NoCaseDeclarations {
  fn new() -> Box<Self> {
    Box::new(NoCaseDeclarations)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoCaseDeclarationsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Requires lexical declarations (`let`, `const`, `function` and `class`) in
switch `case` or `default` clauses to be scoped with brackets.

Without brackets in the `case` or `default` block, the lexical declarations are
visible to the entire switch block but only get initialized when they are assigned,
which only happens if that case/default is reached.  This can lead to unexpected
errors.  The solution is to ensure each `case` or `default` block is wrapped in
brackets to scope limit the declarations.

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
```

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
"#
  }
}

struct NoCaseDeclarationsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoCaseDeclarationsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoCaseDeclarationsVisitor<'c, 'view> {
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
          CODE,
          MESSAGE,
          HINT,
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
    assert_lint_err! {
      NoCaseDeclarations,
      r#"
switch (foo) {
  case 1:
    let a = "a";
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (bar) {
  default:
    let a = "a";
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (fizz) {
  case 1:
    const a = "a";
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (buzz) {
  default:
    const a = "a";
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (fncase) {
  case 1:
    function fn() {

    }
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (fncase) {
  default:
    function fn() {

    }
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (classcase) {
  case 1:
    class Cl {

    }
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
switch (classcase) {
  default:
    class Cl {

    }
    break;
}
    "#: [
        {
          line: 3,
          col: 2,
          message: MESSAGE,
          hint: HINT,
        }
      ],

      // nested switch
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
    "#: [
        {
          line: 5,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
    "#: [
        {
          line: 5,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
    "#: [
        {
          line: 5,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ],
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
    "#: [
        {
          line: 5,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
