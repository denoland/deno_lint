// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use std::collections::HashSet;
use swc_common::Span;
use swc_ecmascript::ast::{Module, ObjectLit};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

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

  fn report(&mut self, span: Span, key: String) {
    self.context.add_diagnostic(
      span,
      "no-dupe-keys",
      format!("Duplicate key '{}'", key),
    );
  }
}

impl<'c> Visit for NoDupeKeysVisitor<'c> {
  noop_visit_type!();

  fn visit_object_lit(&mut self, obj_lit: &ObjectLit, _parent: &dyn Node) {
    let mut keys: HashSet<String> = HashSet::new();

    for prop in &obj_lit.props {
      if let Some(key) = prop.get_key() {
        if keys.contains(&key) {
          self.report(obj_lit.span, key);
        } else {
          keys.insert(key);
        }
      }
    }
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
    assert_lint_ok::<NoDupeKeys>(r#"var foo = { bar: "baz", boo: "bang" }"#);
    assert_lint_ok::<NoDupeKeys>(
      r#"var foo = { bar: "baz", boo: { bar: "bang", }, }"#,
    );
    assert_lint_ok::<NoDupeKeys>(r#"var foo = { __proto__: 1, two: 2};"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { '': 1, bar: 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { '': 1, ' ': 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { '': 1, [null]: 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { '': 1, [a]: 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { [a]: 1, [a]: 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"+{ get a() { }, set a(b) { } };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { a: b, [a]: b };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { a: b, ...c }"#);
    assert_lint_ok::<NoDupeKeys>(
      r#"var x = { get a() {}, set a (value) {} };"#,
    );
    assert_lint_ok::<NoDupeKeys>(r#"var x = ({ null: 1, [/(?<zero>0)/]: 2 })"#);
    assert_lint_ok::<NoDupeKeys>(r#"var {a, a} = obj"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { 012: 1, 12: 2 };"#);
    assert_lint_ok::<NoDupeKeys>(r#"var x = { 1_0: 1, 1: 2 };"#);
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
  }
}
