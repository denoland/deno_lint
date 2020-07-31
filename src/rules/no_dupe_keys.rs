// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{Module, ObjectLit};
use crate::swc_util::Key;
use std::collections::{BTreeSet, HashSet};
use swc_ecmascript::visit::{Node, Visit};

use std::sync::Arc;

pub struct NoDupeKeys;

impl LintRule for NoDupeKeys {
  fn new() -> Box<Self> {
    Box::new(NoDupeKeys)
  }

  fn code(&self) -> &'static str {
    "no-dupe-keys"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoDupeKeysVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDupeKeysVisitor {
  context: Arc<Context>,
}

impl NoDupeKeysVisitor {
  pub fn new(context: Arc<Context>) -> Self {
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
        "no-dupe-keys",
        format!("Duplicate key '{}'", key).as_str(),
      );
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
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_multiple_duplicate_keys() {
    assert_lint_err_n::<NoDupeKeys>(
      r#"var foo = { bar: "baz", bar: "qux", quux: "boom", quux: "bang" };"#,
      vec![10, 10],
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_string_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", "bar": "qux" };"#,
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_numeric_keys() {
    assert_lint_err::<NoDupeKeys>(r#"var foo = { 1: "baz", 0x1: "qux" };"#, 10);
  }

  #[test]
  fn it_fails_when_there_are_duplicate_getter_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", get bar() {} };"#,
      10,
    );
  }

  #[test]
  fn it_fails_when_there_are_duplicate_setter_keys() {
    assert_lint_err::<NoDupeKeys>(
      r#"var foo = { bar: "baz", set bar() {} };"#,
      10,
    );
  }
}
