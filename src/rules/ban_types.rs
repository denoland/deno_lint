// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use swc_ecmascript::ast::{
  TsEntityName, TsKeywordType, TsKeywordTypeKind, TsTypeParamInstantiation,
  TsTypeRef,
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct BanTypes;

impl LintRule for BanTypes {
  fn new() -> Box<Self> {
    Box::new(BanTypes)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "ban-types"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = BanTypesVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Bans the use of primitive wrapper objects (e.g. `String` the object is a 
wrapper of `string` the primitive) in addition to the non-explicit `Function`
type and the misunderstood `Object` type. 

There are very few situations where primitive wrapper objects are desired and
far more often a mistake was made with the case of the primitive type.  You also 
cannot assign a primitive wrapper object to a primitive leading to type issues
down the line.  

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

struct BanTypesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> BanTypesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

static BAN_TYPES_MESSAGE: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(
  || {
    let mut map = HashMap::new();
    map.insert("String", "Use `string` instead");
    map.insert("Boolean", "Use `boolean` instead");
    map.insert("Number", "Use `number` instead");
    map.insert("Symbol", "Use `symbol` instead");
    map.insert("Function", "Define the function shape Explicitly.");
    map.insert("Object",
  "if you want a type meaning `any object` use `Record<string, unknown>` instead,
or if you want a type meaning `any value`, you probably want `unknown` instead.");
    map.insert("object", "Use `Record<string, unknown>` instead");
    map
  },
);

fn get_message(ident: impl AsRef<str>) -> Option<&'static str> {
  BAN_TYPES_MESSAGE.get(ident.as_ref()).copied()
}

impl<'c> Visit for BanTypesVisitor<'c> {
  fn visit_ts_type_ref(&mut self, ts_type_ref: &TsTypeRef, _parent: &dyn Node) {
    if let TsEntityName::Ident(ident) = &ts_type_ref.type_name {
      if let Some(message) = get_message(&ident.sym) {
        self
          .context
          .add_diagnostic(ts_type_ref.span, "ban-types", message);
      }
    }
    if let Some(type_param) = &ts_type_ref.type_params {
      self.visit_ts_type_param_instantiation(type_param, ts_type_ref);
    }
  }

  fn visit_ts_keyword_type(
    &mut self,
    ts_keyword_type: &TsKeywordType,
    _parent: &dyn Node,
  ) {
    if TsKeywordTypeKind::TsObjectKeyword == ts_keyword_type.kind {
      self.context.add_diagnostic(
        ts_keyword_type.span,
        "ban-types",
        get_message("object").unwrap(), // `BAN_TYPES_MESSAGE` absolutely has `object` key
      );
    }
  }

  fn visit_ts_type_param_instantiation(
    &mut self,
    ts_type_param_instantiation: &TsTypeParamInstantiation,
    _parent: &dyn Node,
  ) {
    for param in ts_type_param_instantiation.params.iter() {
      self.visit_ts_type(&param, ts_type_param_instantiation);
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
      get_message(ty).unwrap()
    }

    assert_lint_err! {
      BanTypes,
      "let a: String;": [
        {
          col: 7,
          message: message("String"),
        }
      ],
      "let a: Object;": [
        {
          col: 7,
          message: message("Object"),
        }
      ],
      "let a: Number;": [
        {
          col: 7,
          message: message("Number"),
        }
      ],
      "let a: Function;": [
        {
          col: 7,
          message: message("Function"),
        }
      ],
      "let a: object;": [
        {
          col: 7,
          message: message("object"),
        }
      ],
      "let a: { b: String };": [
        {
          col: 12,
          message: message("String"),
        }
      ],
      "let a: { b: Number };": [
        {
          col: 12,
          message: message("Number"),
        }
      ],
      "let a: { b: object, c: Object };": [
        {
          col: 12,
          message: message("object"),
        },
        {
          col: 23,
          message: message("Object"),
        }
      ],
      "let a: { b: { c : Function } };": [
        {
          col: 18,
          message: message("Function"),
        }
      ],
      "let a: Array<String>": [
        {
          col: 13,
          message: message("String"),
        }
      ],
      "let a: Number<Function>": [
        {
          col: 7,
          message: message("Number"),
        },
        {
          col: 14,
          message: message("Function"),
        }
      ],
      "function foo(a: String) {}": [
        {
          col: 16,
          message: message("String"),
        }
      ],
      "function foo(): Number {}": [
        {
          col: 16,
          message: message("Number"),
        }
      ],
      "let a: () => Number;": [
        {
          col: 13,
          message: message("Number"),
        }
      ],
      "'a' as String;": [
        {
          col: 7,
          message: message("String"),
        }
      ],
      "1 as Number;": [
        {
          col: 5,
          message: message("Number"),
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
        },
        {
          line: 2,
          col: 34,
          message: message("String"),
        },
        {
          line: 2,
          col: 57,
          message: message("Object"),
        },
        {
          line: 3,
          col: 19,
          message: message("String"),
        },
        {
          line: 3,
          col: 28,
          message: message("Object"),
        },
        {
          line: 5,
          col: 16,
          message: message("String"),
        },
        {
          line: 6,
          col: 15,
          message: message("String"),
        },
        {
          line: 6,
          col: 29,
          message: message("String"),
        }
      ]
    };
  }
}
