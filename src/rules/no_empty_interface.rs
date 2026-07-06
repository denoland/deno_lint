// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Program, TSInterfaceDeclaration};
use derive_more::Display;

#[derive(Debug)]
pub struct NoEmptyInterface;

const CODE: &str = "no-empty-interface";

#[derive(Display)]
enum NoEmptyInterfaceMessage {
  #[display(fmt = "An empty interface is equivalent to `{{}}`.")]
  EmptyObject,
}

#[derive(Display)]
enum NoEmptyInterfaceHint {
  #[display(fmt = "Remove this interface or add members to this interface.")]
  RemoveOrAddMember,
}

impl LintRule for NoEmptyInterface {
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
    let mut handler = NoEmptyInterfaceHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoEmptyInterfaceHandler;

impl Handler<'_> for NoEmptyInterfaceHandler {
  fn ts_interface_declaration(
    &mut self,
    interface_decl: &TSInterfaceDeclaration,
    ctx: &mut Context,
  ) {
    if interface_decl.extends.is_empty() && interface_decl.body.body.is_empty()
    {
      ctx.add_diagnostic_with_hint(
        interface_decl.span,
        CODE,
        NoEmptyInterfaceMessage::EmptyObject,
        NoEmptyInterfaceHint::RemoveOrAddMember,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_interface_valid() {
    assert_lint_ok! {
      NoEmptyInterface,
      "interface Foo { a: string }",
      "interface Foo { a: number }",

      // This is valid, because:
      //  - `Bar` can be a type, this makes it so `Foo` has the same members
      //    as `Bar` but is an interface instead. Behaviour of types and interfaces
      //    isn't always the same.
      //  - `Foo` interface might already exist and extend it by the `Bar` members.
      "interface Foo extends Bar {}",

      // This is valid because an interface with more than one supertype
      // can be used as a replacement of a union type.
      "interface Foo extends Bar, Baz {}",
    };
  }

  #[test]
  fn no_empty_interface_invalid() {
    assert_lint_err! {
      NoEmptyInterface,
      "interface Foo {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::EmptyObject,
          hint: NoEmptyInterfaceHint::RemoveOrAddMember,
        }
      ],
    };
  }
}
