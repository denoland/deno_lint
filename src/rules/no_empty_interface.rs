// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::TsInterfaceDecl;
use deno_ast::SourceRanged;
use derive_more::Display;

#[derive(Debug)]
pub struct NoEmptyInterface;

const CODE: &str = "no-empty-interface";

#[derive(Display)]
enum NoEmptyInterfaceMessage {
  #[display("An empty interface is equivalent to `{{}}`.")]
  EmptyObject,
}

#[derive(Display)]
enum NoEmptyInterfaceHint {
  #[display("Remove this interface or add members to this interface.")]
  RemoveOrAddMember,
}

impl LintRule for NoEmptyInterface {
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
    NoEmptyInterfaceHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_empty_interface.md")
  }
}

struct NoEmptyInterfaceHandler;

impl Handler for NoEmptyInterfaceHandler {
  fn ts_interface_decl(
    &mut self,
    interface_decl: &TsInterfaceDecl,
    ctx: &mut Context,
  ) {
    if interface_decl.extends.is_empty() && interface_decl.body.body.is_empty()
    {
      ctx.add_diagnostic_with_hint(
        interface_decl.range(),
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
      "interface Foo extends {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::EmptyObject,
          hint: NoEmptyInterfaceHint::RemoveOrAddMember,
        }
      ],
    };
  }
}
