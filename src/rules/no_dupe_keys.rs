// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  GetterProp, KeyValueProp, MethodProp, ObjectLit, Prop, PropOrSpread,
  SetterProp,
};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Debug)]
pub struct NoDupeKeys;

const CODE: &str = "no-dupe-keys";

#[derive(Display)]
enum NoDupeKeysMessage {
  #[display(fmt = "Duplicate key '{}'", _0)]
  Duplicate(String),
}

#[derive(Display)]
enum NoDupeKeysHint {
  #[display(fmt = "Remove or rename the duplicate key")]
  RemoveOrRename,
}

impl LintRule for NoDupeKeys {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoDupeKeysHandler.traverse(program, context);
  }
}

struct NoDupeKeysHandler;

impl NoDupeKeysHandler {
  fn report(
    &mut self,
    range: SourceRange,
    key: impl Into<String>,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      NoDupeKeysMessage::Duplicate(key.into()),
      NoDupeKeysHint::RemoveOrRename,
    );
  }

  fn check_key<S: Into<String>>(
    &mut self,
    obj_range: SourceRange,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
    ctx: &mut Context,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(occupied) => {
          self.report(obj_range, occupied.key(), ctx);
        }
        Entry::Vacant(vacant) => {
          vacant.insert(PropertyInfo::default());
        }
      }
    }
  }

  fn check_getter<S: Into<String>>(
    &mut self,
    obj_range: SourceRange,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
    ctx: &mut Context,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(mut occupied) => {
          if occupied.get().setter_only() {
            occupied.get_mut().getter = true;
          } else {
            self.report(obj_range, occupied.key(), ctx);
          }
        }
        Entry::Vacant(vacant) => {
          vacant.insert(PropertyInfo {
            getter: true,
            setter: false,
          });
        }
      }
    }
  }

  fn check_setter<S: Into<String>>(
    &mut self,
    obj_range: SourceRange,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
    ctx: &mut Context,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(mut occupied) => {
          if occupied.get().getter_only() {
            occupied.get_mut().setter = true;
          } else {
            self.report(obj_range, occupied.key(), ctx);
          }
        }
        Entry::Vacant(vacant) => {
          vacant.insert(PropertyInfo {
            getter: false,
            setter: true,
          });
        }
      }
    }
  }
}

#[derive(Clone, Copy, Default)]
struct PropertyInfo {
  getter: bool,
  setter: bool,
}

impl PropertyInfo {
  fn getter_only(&self) -> bool {
    self.getter && !self.setter
  }

  fn setter_only(&self) -> bool {
    self.setter && !self.getter
  }
}

impl Handler for NoDupeKeysHandler {
  fn object_lit(&mut self, obj_lit: &ObjectLit, ctx: &mut Context) {
    let range = obj_lit.range();
    let mut keys: HashMap<String, PropertyInfo> = HashMap::new();

    for prop in obj_lit.props {
      if let PropOrSpread::Prop(prop) = prop {
        match prop {
          Prop::Shorthand(ident) => {
            self.check_key(range, Some(ident.inner.as_ref()), &mut keys, ctx);
          }
          Prop::KeyValue(KeyValueProp { key, .. }) => {
            self.check_key(range, key.string_repr(), &mut keys, ctx);
          }
          Prop::Assign(_) => {}
          Prop::Getter(GetterProp { key, .. }) => {
            self.check_getter(range, key.string_repr(), &mut keys, ctx);
          }
          Prop::Setter(SetterProp { key, .. }) => {
            self.check_setter(range, key.string_repr(), &mut keys, ctx);
          }
          Prop::Method(MethodProp { key, .. }) => {
            self.check_key(range, key.string_repr(), &mut keys, ctx);
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.11.0/tests/lib/rules/no-dupe-keys.js
  // MIT Licensed.

  #[test]
  fn no_dupe_keys_valid() {
    assert_lint_ok! {
      NoDupeKeys,
      r#"var foo = { bar: "baz", boo: "bang" }"#,
      r#"var foo = { bar: "baz", boo: { bar: "bang", }, }"#,
      r#"var foo = { __proto__: 1, two: 2};"#,
      r#"var x = { '': 1, bar: 2 };"#,
      r#"var x = { '': 1, ' ': 2 };"#,
      r#"var x = { '': 1, [null]: 2 };"#,
      r#"var x = { '': 1, [a]: 2 };"#,
      r#"var x = { [a]: 1, [a]: 2 };"#,
      r#"+{ get a() { }, set a(b) { } };"#,
      r#"var x = { a: b, [a]: b };"#,
      r#"var x = { a: b, ...c }"#,
      r#"var x = { get a() {}, set a (value) {} };"#,
      r#"var x = ({ null: 1, [/(?<zero>0)/]: 2 })"#,
      r#"var {a, a} = obj"#,
      r#"var x = { 012: 1, 12: 2 };"#,
      r#"var x = { 1_0: 1, 1: 2 };"#,
      // nested
      r#"
let x = {
  y: {
    foo: 0,
    bar: 1,
  },
};
"#,
    };
  }

  #[test]
  fn no_dupe_keys_invalid() {
    assert_lint_err! {
      NoDupeKeys,
      r#"var foo = { bar: "baz", bar: "qux" };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var foo = { bar: "baz", bar: "qux", quux: "boom", quux: "bang" };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        },
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "quux"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var foo = { bar: "baz", "bar": "qux" };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var foo = { 1: "baz", 0x1: "qux" };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "1"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var foo = { bar: "baz", get bar() {} };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var foo = { bar: "baz", set bar() {} };"#: [
        {
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { a: b, ['a']: b };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "a"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { '': 1, '': 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, ""),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { '': 1, [``]: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, ""),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { 012: 1, 10: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "10"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { 0b1: 1, 1: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "1"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { 0o1: 1, 1: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "1"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { 1n: 1, 1: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "1"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { 1_0: 1, 10: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "10"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { "z": 1, z: 2 };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "z"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"
var foo = {
  bar: 1,
  bar: 1,
}
"#: [
        {
          line: 2,
          col: 10,
          message: variant!(NoDupeKeysMessage, Duplicate, "bar"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = { a: 1, b: { a: 2 }, get b() {} };"#: [
        {
          col: 8,
          message: variant!(NoDupeKeysMessage, Duplicate, "b"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],
      r#"var x = ({ '/(?<zero>0)/': 1, [/(?<zero>0)/]: 2 })"#: [
        {
          col: 9,
          message: variant!(NoDupeKeysMessage, Duplicate, "/(?<zero>0)/"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ],

      // nested
      r#"
let x = {
  key: {
    dup: 0,
    dup: 1,
  },
};
"#: [
        {
          line: 3,
          col: 7,
          message: variant!(NoDupeKeysMessage, Duplicate, "dup"),
          hint: NoDupeKeysHint::RemoveOrRename,
        }
      ]
    };
  }
}
