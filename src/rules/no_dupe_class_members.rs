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

pub struct NoDupeClassMembers;

impl LintRule for NoDupeClassMembers {
  fn new() -> Box<Self> {
    Box::new(NoDupeClassMembers)
  }

  fn code(&self) -> &'static str {
    "noDupeClassMembers"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoDupeClassMembersVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct ClassMethodDescriptor {
  method: bool,
  getter: bool,
  setter: bool,
}

type ClassMethodMap = HashMap<String, String>;

pub struct NoDupeClassMembersVisitor {
  context: Context,
  // Because class can be defined inside another class
  // we need to keep track of nested classes
  class_stack: Vec<ClassMethodMap>,
}

impl NoDupeClassMembersVisitor {
  pub fn new(context: Context) -> Self {
    Self { context, class_stack: vec![] }
  }
}

impl Visit for NoDupeClassMembersVisitor {
  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let class_map = {};
    self.class_stack.push(class_map);
    swc_ecma_visit::visit_class(self, class, parent);
    self.class_stack.pop();
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod, parent: &dyn Node) {
    let method_name = class_method.key.to_string();

    let class_map = self.class_stack.last_mut().unwrap();
    let (descriptor, static_descriptor) = class_map.entry(method_name).or_insert(
      (ClassMethodDescriptor::default(), ClassMethodDescriptor::default())
    );

    match class_method.kind {
      MethodKind::Method => {
        descriptor.method = true;
      },
      MethodKind::Getter => {
        descriptor.getter = true;
      },
      MethodKind::Setter => {
        descriptor.setter = true;
      },
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
