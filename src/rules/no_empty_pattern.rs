// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrayPattern, AssignmentPattern, FormalParameter, ObjectPattern, Program,
};
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::syntax::scope::ScopeFlags;

#[derive(Debug)]
pub struct NoEmptyPattern;

const CODE: &str = "no-empty-pattern";
const MESSAGE: &str = "empty patterns are not allowed";
const HINT: &str =
  "Add variable to pattern or apply correct default value syntax with `=`";

impl LintRule for NoEmptyPattern {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut visitor = NoEmptyPatternVisitor {
      context,
      in_assignment_pattern_left: false,
    };
    visitor.visit_program(program);
  }
}

struct NoEmptyPatternVisitor<'c, 'a> {
  context: &'c mut Context<'a>,
  /// True when we're visiting the left side of an AssignmentPattern.
  /// An empty pattern with a default value (`{} = defaultVal`) is allowed.
  in_assignment_pattern_left: bool,
}

impl<'a> Visit<'a> for NoEmptyPatternVisitor<'_, 'a> {
  fn visit_assignment_pattern(&mut self, n: &AssignmentPattern<'a>) {
    let prev = self.in_assignment_pattern_left;
    self.in_assignment_pattern_left = true;
    self.visit_binding_pattern(&n.left);
    self.in_assignment_pattern_left = prev;
    self.visit_expression(&n.right);
  }

  fn visit_formal_parameter(&mut self, n: &FormalParameter<'a>) {
    // In OXC, function parameter defaults are in `initializer`, not nested in
    // an AssignmentPattern. Treat any parameter with an initializer as having
    // a default value, so an empty pattern like `{}: Type = {}` is allowed.
    let prev = self.in_assignment_pattern_left;
    if n.initializer.is_some() {
      self.in_assignment_pattern_left = true;
    }
    walk::walk_formal_parameter(self, n);
    self.in_assignment_pattern_left = prev;
  }

  fn visit_object_pattern(&mut self, obj_pat: &ObjectPattern<'a>) {
    if obj_pat.properties.is_empty()
      && obj_pat.rest.is_none()
      && !self.in_assignment_pattern_left
    {
      self
        .context
        .add_diagnostic_with_hint(obj_pat.span, CODE, MESSAGE, HINT);
    }
    walk::walk_object_pattern(self, obj_pat);
  }

  fn visit_array_pattern(&mut self, arr_pat: &ArrayPattern<'a>) {
    if arr_pat.elements.is_empty()
      && arr_pat.rest.is_none()
      && !self.in_assignment_pattern_left
    {
      self
        .context
        .add_diagnostic_with_hint(arr_pat.span, CODE, MESSAGE, HINT);
    }
    walk::walk_array_pattern(self, arr_pat);
  }

  fn visit_function(
    &mut self,
    func: &deno_ast::oxc::ast::ast::Function<'a>,
    flags: ScopeFlags,
  ) {
    walk::walk_function(self, func, flags);
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
