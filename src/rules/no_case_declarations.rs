// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{Decl, Stmt, SwitchCase, VarDeclKind};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoCaseDeclarations;

const CODE: &str = "no-case-declarations";
const MESSAGE: &str = "Unexpected declaration in case";
const HINT: &str = "Wrap switch case and default blocks in brackets";

impl LintRule for NoCaseDeclarations {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoCaseDeclarationsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_case_declarations.md")
  }
}

struct NoCaseDeclarationsHandler;

impl Handler for NoCaseDeclarationsHandler {
  fn switch_case(&mut self, switch_case: &SwitchCase, context: &mut Context) {
    for stmt in &switch_case.cons {
      let is_lexical_decl = match stmt {
        Stmt::Decl(decl) => match &decl {
          Decl::Fn(_) => true,
          Decl::Class(_) => true,
          Decl::Var(var_decl) => var_decl.decl_kind() != VarDeclKind::Var,
          _ => false,
        },
        _ => false,
      };

      if is_lexical_decl {
        context.add_diagnostic_with_hint(
          switch_case.range(),
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
