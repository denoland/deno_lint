// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{ArrayPat, ObjectPat, ObjectPatProp};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyPattern;

impl LintRule for NoEmptyPattern {
  fn new() -> Box<Self> {
    Box::new(NoEmptyPattern)
  }

  fn code(&self) -> &'static str {
    "no-empty-pattern"
  }

  fn lint_module(&self, context: Context, module: &swc_ecma_ast::Module) {
    let mut visitor = NoEmptyPatternVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoEmptyPatternVisitor {
  context: Context,
}

impl NoEmptyPatternVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyPatternVisitor {
  fn visit_object_pat_prop(
    &mut self,
    obj_pat_prop: &ObjectPatProp,
    _parent: &dyn Node,
  ) {
    if let ObjectPatProp::KeyValue(kv_prop) = obj_pat_prop {
      if let swc_ecma_ast::Pat::Object(obj_pat) = &*kv_prop.value {
        self.visit_object_pat(obj_pat, _parent);
      } else if let swc_ecma_ast::Pat::Array(arr_pat) = &*kv_prop.value {
        self.visit_array_pat(arr_pat, _parent);
      }
    }
  }

  fn visit_object_pat(&mut self, obj_pat: &ObjectPat, _parent: &dyn Node) {
    if obj_pat.props.is_empty() {
      if obj_pat.type_ann.is_none() {
        self.context.add_diagnostic(
          obj_pat.span,
          "no-empty-pattern",
          "empty patterns are not allowed",
        )
      }
    } else {
      for prop in &obj_pat.props {
        self.visit_object_pat_prop(prop, _parent)
      }
    }
  }

  fn visit_array_pat(&mut self, arr_pat: &ArrayPat, _parent: &dyn Node) {
    if arr_pat.elems.is_empty() {
      self.context.add_diagnostic(
        arr_pat.span,
        "no-empty-pattern",
        "empty patterns are not allowed",
      )
    } else {
      for elem in &arr_pat.elems {
        if let Some(element) = elem {
          if let swc_ecma_ast::Pat::Object(obj_pat) = element {
            self.visit_object_pat(&obj_pat, _parent);
          } else if let swc_ecma_ast::Pat::Array(arr_pat) = element {
            self.visit_array_pat(&arr_pat, _parent);
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_pattern_valid() {
    assert_lint_ok_n::<NoEmptyPattern>(vec![
      "const {a = {}} = foo;",
      "const {a, b = {}} = foo;",
      "const {a = []} = foo;",
      "function foo({a = {}}) {}",
      "function foo({a = []}) {}",
      "var [a] = foo",
      "async function startFileServerAsLibrary({}: FileServerCfg = {}): Promise<void>",
    ]);
  }

  #[test]
  fn no_empty_pattern_invalid() {
    assert_lint_err::<NoEmptyPattern>("const {} = foo", 6);
    assert_lint_err::<NoEmptyPattern>("const [] = foo", 6);
    assert_lint_err::<NoEmptyPattern>("const {a: {}} = foo", 10);
    assert_lint_err::<NoEmptyPattern>("const {a, b: {}} = foo", 13);
    assert_lint_err::<NoEmptyPattern>("const {a: []} = foo", 10);
    assert_lint_err::<NoEmptyPattern>("function foo({}) {}", 13);
    assert_lint_err::<NoEmptyPattern>("function foo([]) {}", 13);
    assert_lint_err::<NoEmptyPattern>("function foo({a: {}}) {}", 17);
    assert_lint_err::<NoEmptyPattern>("function foo({a: []}) {}", 17);
  }
}
