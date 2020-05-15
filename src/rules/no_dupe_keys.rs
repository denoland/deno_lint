// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::LintRule;
use crate::rules::Context;
use swc_ecma_ast::{Module, ObjectLit};
use swc_ecma_visit::{Visit, Node};
use swc_ecma_ast::PropOrSpread::{Prop,Spread};
use swc_ecma_ast::Prop::{KeyValue, Shorthand};
use swc_ecma_ast::PropName::{Ident,Str};
use std::collections::HashMap;

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
    // let mut count = HashMap::new();
    for prop in &obj_lit.props {

      match prop {
        Prop(p) => {
          println!("{:?}", p);
          match &**p { // Yuck
            Shorthand(s) => {
              // s.sym
            },
            KeyValue(kv) => {
              println!("{:?}", kv.key);
              match &kv.key {
                Ident(i) => {
                  println!("{:?}", i.sym);
                },
                Str(s) => {
                  println!("{:?}", s.value);
                }
                _ => {}
              }
            },
            _ => {},
          }
        },
        Spread(s) => {
          println!("Spread: {:?}", s);
        },
      }
    }

    self.context.add_diagnostic(
      obj_lit.span,
      "noDupeKeys",
      "Duplicate keys are not allowed",
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::test_lint;
  use serde_json::json;

  #[test]
  fn no_dupe_keys_test() {
    test_lint(
      "no_dupe_keys",
      r#"
var foo = {
    bar: "baz",
    "bar": "qux"
};
      "#,
      vec!(NoDupeKeys::new()),
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
}