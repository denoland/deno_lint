// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::{BTreeSet, HashSet};
use swc_ecma_ast::Prop;
use swc_ecma_ast::Prop::*;
use swc_ecma_ast::PropName;
use swc_ecma_ast::PropName::*;
use swc_ecma_ast::PropOrSpread::{Prop as PropVariant, Spread};
use swc_ecma_ast::{Module, ObjectLit, PropOrSpread};
use swc_ecma_visit::{Node, Visit};

pub struct NoDupeKeys;

impl LintRule for NoDupeKeys {
  fn new() -> Box<Self> {
    Box::new(NoDupeKeys)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoDupeKeysVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoDupeKeysVisitor {
  context: Context,
}

impl NoDupeKeysVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoDupeKeysVisitor {
  fn visit_object_lit(&mut self, obj_lit: &ObjectLit, _parent: &dyn Node) {
    let mut keys: HashSet<String> = HashSet::new();
    let mut duplicates: BTreeSet<String> = BTreeSet::new();

    for prop in &obj_lit.props {
      if let Some(key) = prop.get_key() {
        if keys.contains(&key) {
          duplicates.insert(key);
        } else {
          keys.insert(key);
        }
      }
    }

    for key in duplicates {
      self.context.add_diagnostic(
        obj_lit.span,
        "noDupeKeys",
        format!("Duplicate key '{}'", key).as_str(),
      );
    }
  }
}

trait Key {
  fn get_key(&self) -> Option<String>;
}

impl Key for PropOrSpread {
  fn get_key(&self) -> Option<String> {
    match self {
      PropVariant(p) => (&**p).get_key(),
      Spread(_) => None,
    }
  }
}

impl Key for Prop {
  fn get_key(&self) -> Option<String> {
    match self {
      KeyValue(key_value) => key_value.key.get_key(),
      Getter(getter) => getter.key.get_key(),
      Setter(setter) => setter.key.get_key(),
      Method(method) => method.key.get_key(),
      Shorthand(_) => None,
      Assign(_) => None,
    }
  }
}

impl Key for PropName {
  fn get_key(&self) -> Option<String> {
    match self {
      Ident(identifier) => Some(identifier.sym.to_string()),
      Str(str) => Some(str.value.to_string()),
      Num(num) => Some(num.to_string()),
      Computed(_) => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn it_passes_when_there_are_no_duplicate_keys() {
    assert_lint_ok::<NoDupeKeys>(r#"var foo = { bar: "baz", boo: "bang" }"#);
  }

  #[test]
  fn it_passes_when_there_are_duplicate_nested_keys() {
    assert_lint_ok::<NoDupeKeys>(
      r#"var foo = { bar: "baz", boo: { bar: "bang", }, }"#,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", bar: "qux" };"#,
      "noDupeKeys",
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_multiple_duplicate_keys() {
    assert_lint_err_n::<NoDupeKeys>(
      r#"var foo = { bar: "baz", bar: "qux", quux: "boom", quux: "bang" };"#,
      vec![("noDupeKeys", 10), ("noDupeKeys", 10)],
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_string_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", "bar": "qux" };"#,
      "noDupeKeys",
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_numeric_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { 1: "baz", 0x1: "qux" };"#,
      "noDupeKeys",
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_getter_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", get bar() {} };"#,
      "noDupeKeys",
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_setter_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", set bar() {} };"#,
      "noDupeKeys",
      10,
    );
  }
}
