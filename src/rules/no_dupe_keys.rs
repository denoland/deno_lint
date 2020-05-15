// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::LintRule;
use super::Context;
use std::collections::HashSet;
use swc_atoms::JsWord;
use swc_ecma_ast::{Module, ObjectLit, PropOrSpread};
use swc_ecma_ast::PropOrSpread::{Prop as PropVariant, Spread};
use swc_ecma_ast::Prop;
use swc_ecma_ast::Prop::*;
use swc_ecma_ast::PropName;
use swc_ecma_ast::PropName::*;
use swc_ecma_visit::{Visit, Node};

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
    let mut keys: HashSet<JsWord> = HashSet::new();
    let mut duplicates: HashSet<JsWord> = HashSet::new();

    for prop in &obj_lit.props {
      if let Some(key) = prop.get_key() {
        if keys.contains(&key) {
          duplicates.insert(key);
        } else {
          keys.insert(key);
        }
      }
    }

    if !duplicates.is_empty() {
      self.context.add_diagnostic(
        obj_lit.span,
        "noDupeKeys",
        "Duplicate keys are not allowed",
      );
    }
  }
}

trait Keys {
  fn get_key(&self) -> Option<JsWord>;
}

impl Keys for PropOrSpread {
  fn get_key(&self) -> Option<JsWord> {
    match self {
      PropVariant(p) => (&**p).get_key(),
      Spread(_) => None
    }
  }
}

impl Keys for Prop {
  fn get_key(&self) -> Option<JsWord> {
    match self {
      Shorthand(identifier) => Some(identifier.sym.clone()),
      KeyValue(key_value) => key_value.key.get_key(),
      Getter(getter) => getter.key.get_key(),
      Setter(setter) => setter.key.get_key(),
      Method(method) => method.key.get_key(),
      Assign(_) => None,
    }
  }
}

impl Keys for PropName {
  fn get_key(&self) -> Option<JsWord> {
    match self {
      Ident(identifier) => Some(identifier.sym.clone()),
      Str(str) => Some(str.value.clone()),
      Num(_) => None,
      Computed(_) => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::{json, Value};

  #[test]
  fn it_passes_when_there_are_no_duplicate_keys() {
    test_rule(
      r#"
var foo = {
  bar: "baz",
  boo: "bang",
}
     "#,
      json!([]),
    )
  }

  #[test]
  fn it_fails_when_there_are_duplicate_keys() {
    test_rule(
      r#"
var foo = {
  bar: "baz",
  bar: "qux"
};
      "#,
      json!([{
        "code": "noDupeKeys",
        "message": "Duplicate keys are not allowed",
        "location": {
          "filename": "no_dupe_keys",
          "line": 2,
          "col": 10,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_when_there_are_duplicate_string_keys() {
    test_rule(
      r#"
var foo = {
  bar: "baz",
  "bar": "qux"
};
      "#,
      json!([{
        "code": "noDupeKeys",
        "message": "Duplicate keys are not allowed",
        "location": {
          "filename": "no_dupe_keys",
          "line": 2,
          "col": 10,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_when_there_are_duplicate_getter_keys() {
    test_rule(
      r#"
var foo = {
  bar: "baz",
  get bar() {},
};
      "#,
      json!([{
        "code": "noDupeKeys",
        "message": "Duplicate keys are not allowed",
        "location": {
          "filename": "no_dupe_keys",
          "line": 2,
          "col": 10,
        }
      }]),
    )
  }

  #[test]
  fn it_fails_when_there_are_duplicate_setter_keys() {
    test_rule(
      r#"
var foo = {
  bar: "baz",
  set bar() {},
};
      "#,
      json!([{
        "code": "noDupeKeys",
        "message": "Duplicate keys are not allowed",
        "location": {
          "filename": "no_dupe_keys",
          "line": 2,
          "col": 10,
        }
      }]),
    )
  }

  fn test_rule(source_code: &str, expected_diagnostics: Value) {
    test_lint(
      "no_dupe_keys",
      source_code,
      vec!(NoDupeKeys::new()),
      expected_diagnostics,
    )
  }
}