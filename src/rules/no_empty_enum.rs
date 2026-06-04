// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Program, TSEnumDeclaration};

#[derive(Debug)]
pub struct NoEmptyEnum;

const CODE: &str = "no-empty-enum";
const MESSAGE: &str = "An empty enum is equivalent to `{}`. Remove this enum or add members to this enum.";

impl LintRule for NoEmptyEnum {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoEmptyEnumHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoEmptyEnumHandler;

impl Handler<'_> for NoEmptyEnumHandler {
  fn ts_enum_declaration(
    &mut self,
    enum_decl: &TSEnumDeclaration,
    ctx: &mut Context,
  ) {
    if enum_decl.body.members.is_empty() {
      ctx.add_diagnostic(enum_decl.span, CODE, MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_enum_valid() {
    assert_lint_ok! {
      NoEmptyEnum,
      "enum Foo { ONE = 'ONE', TWO = 'TWO' }",
      "const enum Foo { ONE = 'ONE' }",
    };
  }

  #[test]
  fn no_empty_enum_invalid() {
    assert_lint_err! {
      NoEmptyEnum,
      "enum Foo {}": [
        {
          col: 0,
          message: MESSAGE,
        }
      ],
      "const enum Foo {}": [
        {
          col: 0,
          message: MESSAGE,
        }
      ],
      r#"
enum Foo {
  One = 1,
  Two = (() => {
    enum Bar {}
    return 42;
  })(),
}
"#: [
        {
          line: 5,
          col: 4,
          message: MESSAGE,
        }
      ],
      "export enum Foo {}": [
        {
          col: 7,
          message: MESSAGE,
        }
      ],
      "export const enum Foo {}": [
        {
          col: 7,
          message: MESSAGE,
        }
      ]
    };
  }
}
