// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{
  self as AstView, Spanned, TsEntityName, TsKeywordTypeKind,
};
use if_chain::if_chain;
use std::convert::TryFrom;

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
}

impl BannedType {
  fn as_message(&self) -> &'static str {
    use BannedType::*;
    match *self {
      String | Boolean | Number | Symbol => "The corresponding lower-case primitive should be used",
      Function => "This provides no type safety because it represents all functions and classes",
      CapitalObject => "This type may be different from what you expect it to be",
      LowerObject => "This type is tricky to use so should be avoided if possible",
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
      _ => Err(()),
    }
  }
}

impl LintRule for BanTypes {
  fn new() -> Box<Self> {
    Box::new(BanTypes)
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
    program: AstView::Program,
  ) {
    BanTypesHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#"Bans the use of primitive wrapper objects (e.g. `String` the object is a
wrapper of `string` the primitive) in addition to the non-explicit `Function`
type and the misunderstood `Object` type.

There are very few situations where primitive wrapper objects are desired and
far more often a mistake was made with the case of the primitive type.  You also
cannot assign a primitive wrapper object to a primitive leading to type issues
down the line. For reference, [the TypeScript handbook] also says we shouldn't
ever use these wrapper objects.

[the TypeScript handbook]: https://www.typescriptlang.org/docs/handbook/declaration-files/do-s-and-don-ts.html#number-string-boolean-symbol-and-object

With `Function`, it is better to explicitly define the entire function
signature rather than use the non-specific `Function` type which won't give you
type safety with the function.

Finally, `Object` means "any non-nullish value" rather than "any object type".
`Record<string, unknown>` is a good choice for a meaning of "any object type".

### Invalid:
```typescript
let a: Boolean;
let b: String;
let c: Number;
let d: Symbol;
let e: Function;
let f: Object;
let g: object;
let h: {};
```

### Valid:
```typescript
let a: boolean;
let b: string;
let c: number;
let d: symbol;
let e: () => number;
let f: Record<string, unknown>;
```
"#
  }
}

struct BanTypesHandler;

impl Handler for BanTypesHandler {
  fn ts_type_ref(
    &mut self,
    ts_type_ref: &AstView::TsTypeRef,
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
    ts_type_lit: &AstView::TsTypeLit,
    ctx: &mut Context,
  ) {
    if ts_type_lit.members.is_empty() {
      ctx.add_diagnostic_with_hint(
        ts_type_lit.span(),
        CODE,
        BannedType::CapitalObject.as_message(),
        BannedType::CapitalObject.as_hint(),
      );
    }
  }

  fn ts_keyword_type(
    &mut self,
    ts_keyword_type: &AstView::TsKeywordType,
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
          message: message("Object"),
          hint: hint("Object"),
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
