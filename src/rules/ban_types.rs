// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::view as ast_view;
use deno_ast::view::{Spanned, TsEntityName, TsKeywordTypeKind};
use if_chain::if_chain;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Debug)]
pub struct BanTypes;

const CODE: &str = "ban-types";

#[derive(Clone, Copy)]
enum BannedType {
  String,
  Boolean,
  Number,
  Symbol,
  Function,
  CapitalObject,
  LowerObject,
  EmptyObjectLiteral,
}

impl BannedType {
  fn as_message(&self) -> &'static str {
    use BannedType::*;
    match *self {
      String | Boolean | Number | Symbol => "The corresponding lower-case primitive should be used",
      Function => "This provides no type safety because it represents all functions and classes",
      CapitalObject => "This type may be different from what you expect it to be",
      LowerObject => "This type is tricky to use so should be avoided if possible",
      EmptyObjectLiteral => "`{}` doesn't mean an empty object, but means any types other than `null` and `undefined`",
    }
  }

  fn as_hint(&self) -> &'static str {
    use BannedType::*;
    match *self {
      String => "Use `string` instead",
      Boolean => "Use `boolean` instead",
      Number => "Use `number` instead",
      Symbol => "Use `symbol` instead",
      Function => "Define the function shape explicitly",
      CapitalObject => {
        r#"If you want a type meaning "any object", use `Record<string, unknown>` instead. Or if you want a type meaning "any value", you probably want `unknown` instead."#
      }
      LowerObject => "Use `Record<string, unknown>` instead",
      EmptyObjectLiteral => {
        r#"If you want a type that means "empty object", use `Record<string, never>` instead"#
      }
    }
  }
}

impl TryFrom<&str> for BannedType {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "String" => Ok(Self::String),
      "Boolean" => Ok(Self::Boolean),
      "Number" => Ok(Self::Number),
      "Symbol" => Ok(Self::Symbol),
      "Function" => Ok(Self::Function),
      "Object" => Ok(Self::CapitalObject),
      "object" => Ok(Self::LowerObject),
      "{}" => Ok(Self::EmptyObjectLiteral),
      _ => Err(()),
    }
  }
}

impl LintRule for BanTypes {
  fn new() -> Arc<Self> {
    Arc::new(BanTypes)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    BanTypesHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_types.md")
  }
}

struct BanTypesHandler;

impl Handler for BanTypesHandler {
  fn ts_type_ref(
    &mut self,
    ts_type_ref: &ast_view::TsTypeRef,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let TsEntityName::Ident(ident) = &ts_type_ref.type_name;
      if let Ok(banned_type) = BannedType::try_from(ident.sym().as_ref());
      then {
        ctx.add_diagnostic_with_hint(
          ts_type_ref.span(),
          CODE,
          banned_type.as_message(),
          banned_type.as_hint(),
        );
      }
    }
  }

  fn ts_type_lit(
    &mut self,
    ts_type_lit: &ast_view::TsTypeLit,
    ctx: &mut Context,
  ) {
    if ts_type_lit.members.is_empty() {
      ctx.add_diagnostic_with_hint(
        ts_type_lit.span(),
        CODE,
        BannedType::EmptyObjectLiteral.as_message(),
        BannedType::EmptyObjectLiteral.as_hint(),
      );
    }
  }

  fn ts_keyword_type(
    &mut self,
    ts_keyword_type: &ast_view::TsKeywordType,
    ctx: &mut Context,
  ) {
    if TsKeywordTypeKind::TsObjectKeyword == ts_keyword_type.keyword_kind() {
      ctx.add_diagnostic_with_hint(
        ts_keyword_type.span(),
        CODE,
        BannedType::LowerObject.as_message(),
        BannedType::LowerObject.as_hint(),
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_types_valid() {
    assert_lint_ok! {
      BanTypes,
      "let f = Object();",
      "let f: { x: number; y: number } = { x: 1, y: 1 };",
      "let f = Object();",
      "let g = Object.create(null);",
      "let h = String(false);",
      "let e: foo.String;",
    };
  }

  #[test]
  fn ban_types_invalid() {
    fn message(ty: &str) -> &str {
      BannedType::try_from(ty).unwrap().as_message()
    }

    fn hint(ty: &str) -> &str {
      BannedType::try_from(ty).unwrap().as_hint()
    }

    assert_lint_err! {
      BanTypes,
      "let a: String;": [
        {
          col: 7,
          message: message("String"),
          hint: hint("String"),
        }
      ],
      "let a: Object;": [
        {
          col: 7,
          message: message("Object"),
          hint: hint("Object"),
        }
      ],
      "let a: Number;": [
        {
          col: 7,
          message: message("Number"),
          hint: hint("Number"),
        }
      ],
      "let a: Function;": [
        {
          col: 7,
          message: message("Function"),
          hint: hint("Function"),
        }
      ],
      "let a: object;": [
        {
          col: 7,
          message: message("object"),
          hint: hint("object"),
        }
      ],
      "let a: {};": [
        {
          col: 7,
          message: message("{}"),
          hint: hint("{}"),
        }
      ],
      "let a: { b: String };": [
        {
          col: 12,
          message: message("String"),
          hint: hint("String"),
        }
      ],
      "let a: { b: Number };": [
        {
          col: 12,
          message: message("Number"),
          hint: hint("Number"),
        }
      ],
      "let a: { b: object, c: Object };": [
        {
          col: 12,
          message: message("object"),
          hint: hint("object"),
        },
        {
          col: 23,
          message: message("Object"),
          hint: hint("Object"),
        }
      ],
      "let a: { b: { c : Function } };": [
        {
          col: 18,
          message: message("Function"),
          hint: hint("Function"),
        }
      ],
      "let a: Array<String>": [
        {
          col: 13,
          message: message("String"),
          hint: hint("String"),
        }
      ],
      "let a: Number<Function>": [
        {
          col: 7,
          message: message("Number"),
          hint: hint("Number"),
        },
        {
          col: 14,
          message: message("Function"),
          hint: hint("Function"),
        }
      ],
      "function foo(a: String) {}": [
        {
          col: 16,
          message: message("String"),
          hint: hint("String"),
        }
      ],
      "function foo(): Number {}": [
        {
          col: 16,
          message: message("Number"),
          hint: hint("Number"),
        }
      ],
      "let a: () => Number;": [
        {
          col: 13,
          message: message("Number"),
          hint: hint("Number"),
        }
      ],
      "'a' as String;": [
        {
          col: 7,
          message: message("String"),
          hint: hint("String"),
        }
      ],
      "1 as Number;": [
        {
          col: 5,
          message: message("Number"),
          hint: hint("Number"),
        }
      ],
      "
class Foo<F = String> extends Bar<String> implements Baz<Object> {
  constructor(foo: String | Object) {}

  exit(): Array<String> {
    const foo: String = 1 as String;
  }
}": [
        {
          line: 2,
          col: 14,
          message: message("String"),
          hint: hint("String"),
        },
        {
          line: 2,
          col: 34,
          message: message("String"),
          hint: hint("String"),
        },
        {
          line: 2,
          col: 57,
          message: message("Object"),
          hint: hint("Object"),
        },
        {
          line: 3,
          col: 19,
          message: message("String"),
          hint: hint("String"),
        },
        {
          line: 3,
          col: 28,
          message: message("Object"),
          hint: hint("Object"),
        },
        {
          line: 5,
          col: 16,
          message: message("String"),
          hint: hint("String"),
        },
        {
          line: 6,
          col: 15,
          message: message("String"),
          hint: hint("String"),
        },
        {
          line: 6,
          col: 29,
          message: message("String"),
          hint: hint("String"),
        }
      ]
    };
  }
}
