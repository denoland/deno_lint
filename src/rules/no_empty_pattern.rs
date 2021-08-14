// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::{ArrayPat, ObjectPat, ObjectPatProp};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoEmptyPattern;

const CODE: &str = "no-empty-pattern";
const MESSAGE: &str = "empty patterns are not allowed";
const HINT: &str =
  "Add variable to pattern or apply correct default value syntax with `=`";

impl LintRule for NoEmptyPattern {
  fn new() -> Box<Self> {
    Box::new(NoEmptyPattern)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoEmptyPatternVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_empty_pattern.md")
  }
}

struct NoEmptyPatternVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoEmptyPatternVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoEmptyPatternVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_object_pat_prop(
    &mut self,
    obj_pat_prop: &ObjectPatProp,
    _parent: &dyn Node,
  ) {
    if let ObjectPatProp::KeyValue(kv_prop) = obj_pat_prop {
      if let swc_ecmascript::ast::Pat::Object(obj_pat) = &*kv_prop.value {
        self.visit_object_pat(obj_pat, _parent);
      } else if let swc_ecmascript::ast::Pat::Array(arr_pat) = &*kv_prop.value {
        self.visit_array_pat(arr_pat, _parent);
      }
    }
  }

  fn visit_object_pat(&mut self, obj_pat: &ObjectPat, _parent: &dyn Node) {
    if obj_pat.props.is_empty() {
      if obj_pat.type_ann.is_none() {
        self
          .context
          .add_diagnostic_with_hint(obj_pat.span, CODE, MESSAGE, HINT)
      }
    } else {
      for prop in &obj_pat.props {
        self.visit_object_pat_prop(prop, _parent)
      }
    }
  }

  fn visit_array_pat(&mut self, arr_pat: &ArrayPat, _parent: &dyn Node) {
    if arr_pat.elems.is_empty() {
      self
        .context
        .add_diagnostic_with_hint(arr_pat.span, CODE, MESSAGE, HINT)
    } else {
      for element in arr_pat.elems.iter().flatten() {
        if let swc_ecmascript::ast::Pat::Object(obj_pat) = element {
          self.visit_object_pat(obj_pat, _parent);
        } else if let swc_ecmascript::ast::Pat::Array(arr_pat) = element {
          self.visit_array_pat(arr_pat, _parent);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_pattern_valid() {
    assert_lint_ok! {
      NoEmptyPattern,
      "const {a = {}} = foo;",
      "const {a, b = {}} = foo;",
      "const {a = []} = foo;",
      "function foo({a = {}}) {}",
      "function foo({a = []}) {}",
      "var [a] = foo",
      "async function startFileServerAsLibrary({}: FileServerCfg = {}): Promise<void>",
    };
  }

  #[test]
  fn no_empty_pattern_invalid() {
    assert_lint_err! {
      NoEmptyPattern,
      "const {} = foo": [{
        col: 6,
        message: MESSAGE,
        hint: HINT,
      }],
      "const [] = foo": [{
        col: 6,
        message: MESSAGE,
        hint: HINT,
      }],
      "const {a: {}} = foo": [{
        col: 10,
        message: MESSAGE,
        hint: HINT,
      }],
      "const {a, b: {}} = foo": [{
        col: 13,
        message: MESSAGE,
        hint: HINT,
      }],
      "const {a: []} = foo": [{
        col: 10,
        message: MESSAGE,
        hint: HINT,
      }],
      "function foo({}) {}": [{
        col: 13,
        message: MESSAGE,
        hint: HINT,
      }],
      "function foo([]) {}": [{
        col: 13,
        message: MESSAGE,
        hint: HINT,
      }],
      "function foo({a: {}}) {}": [{
        col: 17,
        message: MESSAGE,
        hint: HINT,
      }],
      "function foo({a: []}) {}": [{
        col: 17,
        message: MESSAGE,
        hint: HINT,
      }],
    }
  }
}
