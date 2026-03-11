// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoMisusedNew;

const CODE: &str = "no-misused-new";

#[derive(Display)]
enum NoMisusedNewMessage {
  #[display(fmt = "Type aliases cannot be constructed, only classes")]
  TypeAlias,
  #[display(fmt = "Interfaces cannot be constructed, only classes")]
  Interface,
  #[display(fmt = "Class cannot have method named `new`.")]
  NewMethod,
}

#[derive(Display)]
enum NoMisusedNewHint {
  #[display(fmt = "Consider using a class, not a type")]
  NotType,
  #[display(fmt = "Consider using a class, not an interface")]
  NotInterface,
  #[display(fmt = "Rename the method")]
  RenameMethod,
}

impl LintRule for NoMisusedNew {
  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoMisusedNewHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }

  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }
}

struct NoMisusedNewHandler;

fn match_parent_type(
  parent_name: &str,
  return_type: &TSTypeAnnotation,
) -> bool {
  if let TSType::TSTypeReference(type_ref) = &return_type.type_annotation {
    if let TSTypeName::IdentifierReference(ident) = &type_ref.type_name {
      return ident.name.as_str() == parent_name;
    }
  }
  false
}

fn is_constructor_keyword(name: &str) -> bool {
  name == "constructor"
}

impl Handler<'_> for NoMisusedNewHandler {
  fn ts_type_alias_declaration(
    &mut self,
    t: &TSTypeAliasDeclaration,
    ctx: &mut Context,
  ) {
    if let TSType::TSTypeLiteral(lit) = &t.type_annotation {
      for member in &lit.members {
        if let TSSignature::TSMethodSignature(signature) = member {
          if let PropertyKey::StaticIdentifier(ident) = &signature.key {
            if is_constructor_keyword(ident.name.as_str()) {
              ctx.add_diagnostic_with_hint(
                ident.span,
                CODE,
                NoMisusedNewMessage::TypeAlias,
                NoMisusedNewHint::NotType,
              );
            }
          }
        }
      }
    }
  }

  fn ts_interface_declaration(
    &mut self,
    n: &TSInterfaceDeclaration,
    ctx: &mut Context,
  ) {
    let parent_name = n.id.name.as_str();
    for member in &n.body.body {
      match member {
        TSSignature::TSMethodSignature(signature) => {
          if let PropertyKey::StaticIdentifier(ident) = &signature.key {
            if is_constructor_keyword(ident.name.as_str()) {
              // constructor
              ctx.add_diagnostic_with_hint(
                signature.span,
                CODE,
                NoMisusedNewMessage::Interface,
                NoMisusedNewHint::NotInterface,
              );
            }
          }
        }
        TSSignature::TSConstructSignatureDeclaration(signature) => {
          if let Some(return_type) = &signature.return_type {
            if match_parent_type(parent_name, return_type) {
              ctx.add_diagnostic_with_hint(
                signature.span,
                CODE,
                NoMisusedNewMessage::Interface,
                NoMisusedNewHint::NotInterface,
              );
            }
          }
        }
        _ => {}
      }
    }
  }

  fn class(&mut self, class: &Class, ctx: &mut Context) {
    let class_name = match &class.id {
      Some(id) => id.name.as_str(),
      None => return,
    };

    for element in &class.body.body {
      if let ClassElement::MethodDefinition(method) = element {
        let method_name = match &method.key {
          PropertyKey::StaticIdentifier(ident) => {
            Cow::Borrowed(ident.name.as_str())
          }
          PropertyKey::StringLiteral(str_) => {
            Cow::Owned(str_.value.to_string())
          }
          _ => continue,
        };

        if method_name.as_ref() != "new" {
          continue;
        }

        if let Some(return_type) = &method.value.return_type {
          if match_parent_type(class_name, return_type) {
            // new
            ctx.add_diagnostic_with_hint(
              method.span,
              CODE,
              NoMisusedNewMessage::NewMethod,
              NoMisusedNewHint::RenameMethod,
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_misused_new_valid() {
    assert_lint_ok! {
      NoMisusedNew,
      "type T = { new(): T }",
      "interface IC { new(): {} }",
      "class C { new(): {} }",
      "class C { constructor(); }",
      "class C { constructor() {} }",
      r#"
    export class Fnv32a extends Fnv32Base<Fnv32a> {
      write(data: Uint8Array): Fnv32a {
        let hash = this.sum32();

        data.forEach((c) => {
          hash ^= c;
          hash = mul32(hash, prime32);
        });

        this._updateState(hash);
        return this;
      }
    }
      "#,
    };
  }

  #[test]
  fn no_misused_new_invalid() {
    assert_lint_err! {
      NoMisusedNew,
      r#"
interface I {
    new(): I;
    constructor(): void;
}
      "#: [
        {
          line: 3,
          col: 4,
          message: NoMisusedNewMessage::Interface,
          hint: NoMisusedNewHint::NotInterface,
        },
        {
          line: 4,
          col: 4,
          message: NoMisusedNewMessage::Interface,
          hint: NoMisusedNewHint::NotInterface,
        }
      ],
      r#"
interface G {
    new<T>(): G<T>;
}
      "#: [
        {
          line: 3,
          col: 4,
          message: NoMisusedNewMessage::Interface,
          hint: NoMisusedNewHint::NotInterface,
        }
      ],
      r#"
class B {
    method() {
        interface T {
            new(): T
        }
    }
}
      "#: [
        {
          line: 5,
          col: 12,
          message: NoMisusedNewMessage::Interface,
          hint: NoMisusedNewHint::NotInterface,
        }
      ],
      r#"
type T = {
    constructor(): void;
}
      "#: [
        {
          line: 3,
          col: 4,
          message: NoMisusedNewMessage::TypeAlias,
          hint: NoMisusedNewHint::NotType,
        }
      ],
      r#"
class C {
    new(): C;
}
      "#: [
        {
          line: 3,
          col: 4,
          message: NoMisusedNewMessage::NewMethod,
          hint: NoMisusedNewHint::RenameMethod,
        }
      ],
      r#"
class A {
  foo() {
    class C {
      new(): C;
    }
  }
}
      "#: [
        {
          line: 5,
          col: 6,
          message: NoMisusedNewMessage::NewMethod,
          hint: NoMisusedNewHint::RenameMethod,
        }
      ]
    };
  }
}
