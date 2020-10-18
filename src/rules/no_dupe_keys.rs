// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use swc_common::Span;
use swc_ecmascript::ast::{
  GetterProp, KeyValueProp, MethodProp, Module, ObjectLit, Prop, PropOrSpread,
  SetterProp,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct NoDupeKeys;

impl LintRule for NoDupeKeys {
  fn new() -> Box<Self> {
    Box::new(NoDupeKeys)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-dupe-keys"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = NoDupeKeysVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows duplicate keys in object literals.

Setting the same key multiple times in an object literal will override other assignments to that key and can cause unexpected behaviour.

### Invalid:
```typescript
const foo = {
  bar: "baz",
  bar: "qux"
};
```
```typescript
const foo = {
  "bar": "baz",
  bar: "qux"
};
```
```typescript
const foo = {
  0x1: "baz",
  1: "qux"
};
```
### Valid:
```typescript
var foo = {
  bar: "baz",
  quxx: "qux"
};
```"#
  }
}

struct NoDupeKeysVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDupeKeysVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn report(&mut self, span: Span, key: impl AsRef<str>) {
    self.context.add_diagnostic(
      span,
      "no-dupe-keys",
      format!("Duplicate key '{}'", key.as_ref()),
    );
  }

  fn check_key<S: Into<String>>(
    &mut self,
    obj_span: Span,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(occupied) => {
          self.report(obj_span, occupied.key());
        }
        Entry::Vacant(vacant) => {
          vacant.insert(PropertyInfo::default());
        }
      }
    }
  }

  fn check_getter<S: Into<String>>(
    &mut self,
    obj_span: Span,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(mut occupied) => {
          if occupied.get().setter_only() {
            occupied.get_mut().getter = true;
          } else {
            self.report(obj_span, occupied.key());
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
    obj_span: Span,
    key: Option<S>,
    keys: &mut HashMap<String, PropertyInfo>,
  ) {
    if let Some(key) = key {
      let key = key.into();

      match keys.entry(key) {
        Entry::Occupied(mut occupied) => {
          if occupied.get().getter_only() {
            occupied.get_mut().setter = true;
          } else {
            self.report(obj_span, occupied.key());
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

impl<'c> Visit for NoDupeKeysVisitor<'c> {
  noop_visit_type!();

  fn visit_object_lit(&mut self, obj_lit: &ObjectLit, _parent: &dyn Node) {
    let span = obj_lit.span;
    let mut keys: HashMap<String, PropertyInfo> = HashMap::new();

    for prop in &obj_lit.props {
      if let PropOrSpread::Prop(prop) = prop {
        match &**prop {
          Prop::Shorthand(ident) => {
            self.check_key(span, Some(ident.as_ref()), &mut keys);
          }
          Prop::KeyValue(KeyValueProp { key, .. }) => {
            self.check_key(span, key.get_key(), &mut keys);
          }
          Prop::Assign(_) => {}
          Prop::Getter(GetterProp { key, .. }) => {
            self.check_getter(span, key.get_key(), &mut keys);
          }
          Prop::Setter(SetterProp { key, .. }) => {
            self.check_setter(span, key.get_key(), &mut keys);
          }
          Prop::Method(MethodProp { key, .. }) => {
            self.check_key(span, key.get_key(), &mut keys);
          }
        }
      }
    }

    obj_lit.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", bar: "qux" };"#,
      10,
    );
    assert_lint_err_n::<NoDupeKeys>(
      r#"var foo = { bar: "baz", bar: "qux", quux: "boom", quux: "bang" };"#,
      vec![10, 10],
    );
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", "bar": "qux" };"#,
      10,
    );
    assert_lint_err::<NoDupeKeys>(r#"var foo = { 1: "baz", 0x1: "qux" };"#, 10);
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", get bar() {} };"#,
      10,
    );
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", set bar() {} };"#,
      10,
    );
    assert_lint_err::<NoDupeKeys>(r#"var x = { a: b, ['a']: b };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { '': 1, '': 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { '': 1, [``]: 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { 012: 1, 10: 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { 0b1: 1, 1: 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { 0o1: 1, 1: 2 };"#, 8);
    // TODO(magurotuna): this leads to panic due to swc error
    // It seems like tsc v4.0.2 cannot handle this either
    // playground: https://www.typescriptlang.org/play?target=99&ts=4.0.2#code/MYewdgzgLgBCBGArGBeGBvAUDGBGMAXDACwBMANJgL4DcQA
    // assert_lint_err::<NoDupeKeys>(r#"var x = { 1n: 1, 1: 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { 1_0: 1, 10: 2 };"#, 8);
    assert_lint_err::<NoDupeKeys>(r#"var x = { "z": 1, z: 2 };"#, 8);
    assert_lint_err_on_line::<NoDupeKeys>(
      r#"
var foo = {
  bar: 1,
  bar: 1,
}
"#,
      2,
      10,
    );
    assert_lint_err::<NoDupeKeys>(
      r#"var x = { a: 1, b: { a: 2 }, get b() {} };"#,
      8,
    );
    assert_lint_err::<NoDupeKeys>(
      r#"var x = ({ '/(?<zero>0)/': 1, [/(?<zero>0)/]: 2 })"#,
      9,
    );

    // nested
    assert_lint_err_on_line::<NoDupeKeys>(
      r#"
let x = {
  key: {
    dup: 0,
    dup: 1,
  },
};
"#,
      3,
      7,
    );
  }
}
