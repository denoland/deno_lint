// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{
  ClassDecl, ClassMember, Expr, Ident, PropName, TsEntityName, TsInterfaceDecl,
  TsType, TsTypeAliasDecl, TsTypeAnn,
  TsTypeElement::{TsConstructSignatureDecl, TsMethodSignature},
};
use deno_ast::SourceRanged;
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
  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoMisusedNewHandler.traverse(program, context);
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_misused_new.md")
  }
}

struct NoMisusedNewHandler;

fn match_parent_type(parent: &Ident, return_type: &TsTypeAnn) -> bool {
  if let TsType::TsTypeRef(type_ref) = return_type.type_ann {
    if let TsEntityName::Ident(ident) = &type_ref.type_name {
      return ident.sym() == parent.sym();
    }
  }

  false
}

fn is_constructor_keyword(ident: &Ident) -> bool {
  *"constructor" == *ident.sym()
}

impl Handler for NoMisusedNewHandler {
  fn ts_type_alias_decl(&mut self, t: &TsTypeAliasDecl, ctx: &mut Context) {
    if let TsType::TsTypeLit(lit) = t.type_ann {
      for member in lit.members {
        if let TsMethodSignature(signature) = &member {
          if let Expr::Ident(ident) = signature.key {
            if is_constructor_keyword(ident) {
              ctx.add_diagnostic_with_hint(
                ident.range(),
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

  fn ts_interface_decl(&mut self, n: &TsInterfaceDecl, ctx: &mut Context) {
    for member in n.body.body {
      match &member {
        TsMethodSignature(signature) => {
          if let Expr::Ident(ident) = signature.key {
            if is_constructor_keyword(ident) {
              // constructor
              ctx.add_diagnostic_with_hint(
                signature.range(),
                CODE,
                NoMisusedNewMessage::Interface,
                NoMisusedNewHint::NotInterface,
              );
            }
          }
        }
        TsConstructSignatureDecl(signature) => {
          if signature.type_ann.is_some()
            && match_parent_type(n.id, signature.type_ann.as_ref().unwrap())
          {
            ctx.add_diagnostic_with_hint(
              signature.range(),
              CODE,
              NoMisusedNewMessage::Interface,
              NoMisusedNewHint::NotInterface,
            );
          }
        }
        _ => {}
      }
    }
  }

  fn class_decl(&mut self, expr: &ClassDecl, ctx: &mut Context) {
    for member in expr.class.body {
      if let ClassMember::Method(method) = member {
        let method_name = match &method.key {
          PropName::Ident(ident) => ident.sym().as_ref(),
          PropName::Str(str_) => str_.value().as_ref(),
          _ => continue,
        };

        if method_name != "new" {
          continue;
        }

        if method.function.return_type.is_some()
          && match_parent_type(
            expr.ident,
            method.function.return_type.as_ref().unwrap(),
          )
        {
          // new
          ctx.add_diagnostic_with_hint(
            method.range(),
            CODE,
            NoMisusedNewMessage::NewMethod,
            NoMisusedNewHint::RenameMethod,
          );
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
