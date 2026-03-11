// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use derive_more::Display;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct NoDupeClassMembers;

const CODE: &str = "no-dupe-class-members";

#[derive(Display)]
enum NoDupeClassMembersMessage {
  #[display(fmt = "Duplicate name '{}'", _0)]
  Duplicate(String),
}

#[derive(Display)]
enum NoDupeClassMembersHint {
  #[display(fmt = "Rename or remove the function with the duplicated name")]
  RenameOrRemove,
}

impl LintRule for NoDupeClassMembers {
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
    let mut handler = NoDupeClassMembersHandler {
      class_stack: vec![],
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct ClassInfo {
  appeared_methods: BTreeMap<MethodToCheck, Vec<(Span, String)>>,
}

struct NoDupeClassMembersHandler {
  class_stack: Vec<ClassInfo>,
}

fn normalize_property_key(key: &PropertyKey) -> Option<String> {
  key.string_repr()
}

struct MethodToCheck {
  normalized_name: String,
  kind: MethodDefinitionKind,
  is_static: bool,
  /// Whether the key was a computed expression (e.g. `['foo']`).
  /// A computed key of `['constructor']` is not the same as the actual
  /// `constructor` keyword even though both normalize to "constructor".
  computed: bool,
}

impl PartialEq for MethodToCheck {
  fn eq(&self, other: &Self) -> bool {
    if self.normalized_name != other.normalized_name {
      return false;
    }
    if self.is_static != other.is_static {
      return false;
    }
    // The actual `constructor` keyword (kind: Constructor, computed: false) is distinct
    // from a computed method named "constructor" (e.g. `['constructor']()`, computed: true).
    // So `['constructor']() {}` and `constructor() {}` are NOT duplicates.
    // But `['constructor']() {}` and `['constructor']() {}` ARE duplicates.
    if self.normalized_name == "constructor"
      && self.computed != other.computed
    {
      return false;
    }
    !matches!(
      (self.kind, other.kind),
      (MethodDefinitionKind::Get, MethodDefinitionKind::Set)
        | (MethodDefinitionKind::Set, MethodDefinitionKind::Get)
    )
  }
}

impl Eq for MethodToCheck {}

impl PartialOrd for MethodToCheck {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for MethodToCheck {
  fn cmp(&self, other: &Self) -> Ordering {
    // For the "constructor" name, computed vs non-computed are different keys
    // (they compare unequal in PartialEq), so we must distinguish them in Ord too.
    let computed_ord = if self.normalized_name == "constructor" {
      self.computed.cmp(&other.computed)
    } else {
      Ordering::Equal
    };
    self
      .normalized_name
      .cmp(&other.normalized_name)
      .then(self.is_static.cmp(&other.is_static))
      .then(computed_ord)
      .then_with(|| match (self.kind, other.kind) {
        (MethodDefinitionKind::Get, MethodDefinitionKind::Set) => {
          Ordering::Less
        }
        (MethodDefinitionKind::Set, MethodDefinitionKind::Get) => {
          Ordering::Greater
        }
        _ => Ordering::Equal,
      })
  }
}

impl Handler<'_> for NoDupeClassMembersHandler {
  fn class(&mut self, _class: &Class, _ctx: &mut Context) {
    self.class_stack.push(ClassInfo {
      appeared_methods: BTreeMap::new(),
    });
  }

  fn class_exit(&mut self, _class: &Class, ctx: &mut Context) {
    if let Some(class_info) = self.class_stack.pop() {
      for entries in class_info.appeared_methods.values() {
        if entries.len() >= 2 {
          for (span, name) in entries {
            ctx.add_diagnostic_with_hint(
              *span,
              CODE,
              NoDupeClassMembersMessage::Duplicate(name.clone()),
              NoDupeClassMembersHint::RenameOrRemove,
            );
          }
        }
      }
    }
  }

  fn method_definition(
    &mut self,
    method: &MethodDefinition,
    _ctx: &mut Context,
  ) {
    // Only count methods with bodies (skip overload signatures)
    if method.value.body.is_none() {
      return;
    }

    let Some(class_info) = self.class_stack.last_mut() else {
      return;
    };

    let Some(normalized_name) = normalize_property_key(&method.key) else {
      return;
    };

    let name = normalized_name.clone();
    let m = MethodToCheck {
      normalized_name,
      kind: method.kind,
      is_static: method.r#static,
      computed: method.computed,
    };

    class_info
      .appeared_methods
      .entry(m)
      .or_default()
      .push((method.span, name));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_dupe_class_members_valid() {
    assert_lint_ok! {
      NoDupeClassMembers,
      r#"
class Foo {
  bar() {}
  qux() {}
}
      "#,
      r#"
class Foo {
  get bar() {}
  set bar(value: number) {}
}
      "#,
      r#"
class Foo {
  static bar() {}
  bar() {}
}
      "#,
      r#"
class Foo {
  static bar() {}
  get bar() {}
  set bar(value: number) {}
}
      "#,
      r#"
class A { foo() {} }
class B { foo() {} }
      "#,
      r#"
class Foo {
  [bar]() {}
  bar() {}
}
      "#,
      r#"
class Foo {
  'bar'() {}
  'baz'() {}
  qux() {}
}
      "#,
      r#"
class Foo {
  *'bar'() {}
  *'baz'() {}
  *qux() {}
}
      "#,
      r#"
class Foo {
  get 'bar'() {}
  get 'baz'() {}
  get qux() {}
}
      "#,
      r#"
class Foo {
  1() {}
  2() {}
}
      "#,
      r#"
class Foo {
  ['bar']() {}
  ['baz']() {}
}
      "#,
      r#"
class Foo {
  [`bar`]() {}
  [`baz`]() {}
}
      "#,
      r#"
class Foo {
  [12]() {}
  [123]() {}
}
      "#,
      r#"
class Foo {
  [1.0]() {}
  ['1.0']() {}
}
      "#,
      r#"
class Foo {
  [0x1]() {}
  [`0x1`]() {}
}
      "#,
      r#"
class Foo {
  [null]() {}
  ['']() {}
}
      "#,
      r#"
class Foo {
  get ['bar']() {}
  set ['bar'](value: number) {}
}
      "#,
      r#"
class Foo {
  ['bar']() {}
  static ['bar']() {}
}
      "#,
      r#"
class Foo {
  ['constructor']() {}
  constructor() {}
}
      "#,
      r#"
class Foo {
  'constructor'() {}
  [`constructor`]() {}
}
      "#,
      r#"
class Foo {
  contrructor() {}
  get [`constructor`]() {}
}
      "#,
      r#"
class Foo {
  contrructor() {}
  set [`constructor`](value: number) {}
}
      "#,
      r#"
class Foo {
  ['bar' + '']() {}
  ['bar']() {}
}
      "#,
      r#"
class Foo {
  [`bar${''}`]() {}
  [`bar`]() {}
}
      "#,
      r#"
class Foo {
  [-1]() {}
  ['-1']() {}
}
      "#,
      r#"
class Foo {
  [foo]() {}
  [foo]() {}
}
      "#,
      r#"
class Foo {
  foo() {
    class Bar {
      foo() {}
    }
    foo();
  }
}
      "#,
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
}
      "#,
    };
  }

  #[test]
  fn no_dupe_class_members_invalid() {
    assert_lint_err! {
      NoDupeClassMembers,
      r#"
class Foo {
  bar() {}
  bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
!class Foo {
  bar() {}
  bar() {}
};
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  'bar'() {}
  'bar'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  10() {}
  1e1() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "10"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "10"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ['bar']() {}
  ['bar']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static ['bar']() {}
  static bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  set 'bar'(value: number) {}
  set ['bar'](val: number) {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ''() {}
  ['']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, ""),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, ""),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [`bar`]() {}
  [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static get [`bar`]() {}
  static get ['bar']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  get [`bar`]() {}
  'bar'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static 'bar'() {}
  static [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ['constructor']() {}
  ['constructor']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static [`constructor`]() {}
  static ['constructor']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123]() {}
  [123]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [0x10]() {}
  16() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "16"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "16"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [100]() {}
  [1e2]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "100"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "100"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123.00]() {}
  [`123`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static '65'() {}
  static [0o101]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "65"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "65"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123n]() {}
  123() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [null]() {}
  'null'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "null"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "null"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  get bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  bar() {}
  get bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 5,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static bar() {}
  static bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  set bar(value: number) {}
  bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  foo() {
    class Bar {
      set bar(value: number) {}
      bar() {}
    }
  }
}
      "#: [
        {
          line: 5,
          col: 6,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 6,
          col: 6,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
  set bar(value: number) {}
}
      "#: [
        {
          line: 5,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 6,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ]
    };
  }
}
