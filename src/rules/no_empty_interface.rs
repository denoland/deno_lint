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
  #[display(fmt = "An empty interface is equivalent to `{{}}`.")]
  EmptyObject,
  #[display(
    fmt = "An interface declaring no members is equivalent to its supertype."
  )]
  Supertype,
}

#[derive(Display)]
enum NoEmptyInterfaceHint {
  #[display(fmt = "Remove this interface or add members to this interface.")]
  RemoveOrAddMember,
  #[display(
    fmt = "Use the supertype instead, or add members to this interface."
  )]
  UseSuperTypeOrAddMember,
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
    if interface_decl.extends.len() <= 1 && interface_decl.body.body.is_empty()
    {
      ctx.add_diagnostic_with_hint(
        interface_decl.range(),
        CODE,
        if interface_decl.extends.is_empty() {
          NoEmptyInterfaceMessage::EmptyObject
        } else {
          NoEmptyInterfaceMessage::Supertype
        },
        if interface_decl.extends.is_empty() {
          NoEmptyInterfaceHint::RemoveOrAddMember
        } else {
          NoEmptyInterfaceHint::UseSuperTypeOrAddMember
        },
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
      r#"
interface Foo {
  a: string;
}

interface Bar extends Foo {}
"#: [
        {
          line: 6,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo extends Array<number> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo extends Array<number | {}> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
interface Foo {
  a: string;
}

interface Bar extends Array<Foo> {}
"#: [
        {
          line: 6,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
type R = Record<string, unknown>;
interface Foo extends R {}
"#: [
        {
          line: 3,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo<T> extends Bar<T> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
declare module FooBar {
  type Baz = typeof baz;
  export interface Bar extends Baz {}
}
"#: [
        {
          line: 4,
          col: 9,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ]
    };
  }
}
