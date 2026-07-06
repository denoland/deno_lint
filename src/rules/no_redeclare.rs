// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, BindingPattern, BlockStatement, ForInStatement,
  ForOfStatement, ForStatement, Function, FunctionType, Program,
  PropertyDefinition, VariableDeclaration, VariableDeclarationKind,
};
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::Span;
use deno_ast::oxc::syntax::scope::ScopeFlags;

use std::collections::HashSet;

#[derive(Debug)]
pub struct NoRedeclare;

const CODE: &str = "no-redeclare";
const MESSAGE: &str = "Redeclaration is not allowed";

impl LintRule for NoRedeclare {
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
    let mut visitor = NoRedeclareVisitor {
      context,
      bindings: Default::default(),
      var_bindings: Default::default(),
    };
    visitor.visit_program(program);
  }
}

struct NoRedeclareVisitor<'c, 'a> {
  context: &'c mut Context<'a>,
  /// Tracks all bindings in the current scope (function or block).
  bindings: HashSet<String>,
  /// Tracks var bindings at function scope (persist across blocks).
  var_bindings: HashSet<String>,
}

impl NoRedeclareVisitor<'_, '_> {
  fn declare(&mut self, name: &str, span: Span) {
    if !self.bindings.insert(name.to_string()) {
      self.context.add_diagnostic(span, CODE, MESSAGE);
    }
  }

  fn declare_var(&mut self, name: &str, span: Span) {
    // `var` declarations are hoisted to function scope; check against var_bindings.
    if !self.var_bindings.insert(name.to_string()) {
      self.context.add_diagnostic(span, CODE, MESSAGE);
    }
    // Also add to current bindings so let/var conflicts are detected.
    self.bindings.insert(name.to_string());
  }

  fn declare_binding_pattern(&mut self, pattern: &BindingPattern<'_>) {
    match pattern {
      BindingPattern::BindingIdentifier(ident) => {
        self.declare(ident.name.as_str(), ident.span);
      }
      BindingPattern::ObjectPattern(obj) => {
        for prop in &obj.properties {
          self.declare_binding_pattern(&prop.value);
        }
        if let Some(rest) = &obj.rest {
          self.declare_binding_pattern(&rest.argument);
        }
      }
      BindingPattern::ArrayPattern(arr) => {
        for elem in arr.elements.iter().flatten() {
          self.declare_binding_pattern(elem);
        }
        if let Some(rest) = &arr.rest {
          self.declare_binding_pattern(&rest.argument);
        }
      }
      BindingPattern::AssignmentPattern(assign) => {
        self.declare_binding_pattern(&assign.left);
      }
    }
  }

  fn declare_var_binding_pattern(&mut self, pattern: &BindingPattern<'_>) {
    match pattern {
      BindingPattern::BindingIdentifier(ident) => {
        self.declare_var(ident.name.as_str(), ident.span);
      }
      BindingPattern::ObjectPattern(obj) => {
        for prop in &obj.properties {
          self.declare_var_binding_pattern(&prop.value);
        }
        if let Some(rest) = &obj.rest {
          self.declare_var_binding_pattern(&rest.argument);
        }
      }
      BindingPattern::ArrayPattern(arr) => {
        for elem in arr.elements.iter().flatten() {
          self.declare_var_binding_pattern(elem);
        }
        if let Some(rest) = &arr.rest {
          self.declare_var_binding_pattern(&rest.argument);
        }
      }
      BindingPattern::AssignmentPattern(assign) => {
        self.declare_var_binding_pattern(&assign.left);
      }
    }
  }
}

impl<'a> Visit<'a> for NoRedeclareVisitor<'_, 'a> {
  fn visit_function(&mut self, f: &Function<'a>, _flags: ScopeFlags) {
    if f.body.is_none() {
      return;
    }

    if f.r#type == FunctionType::FunctionDeclaration {
      if let Some(id) = &f.id {
        self.declare(id.name.as_str(), id.span);
      }
    }

    let parent_bindings = std::mem::take(&mut self.bindings);
    let parent_var_bindings = std::mem::take(&mut self.var_bindings);

    // Declare params in new scope
    for param in &f.params.items {
      self.declare_binding_pattern(&param.pattern);
    }
    // Params also go into var_bindings (they're function-scoped)
    for param in &f.params.items {
      collect_binding_pattern_names(&param.pattern, &mut self.var_bindings);
    }

    if let Some(body) = &f.body {
      for stmt in &body.statements {
        self.visit_statement(stmt);
      }
    }

    self.bindings = parent_bindings;
    self.var_bindings = parent_var_bindings;
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    let parent_bindings = std::mem::take(&mut self.bindings);
    let parent_var_bindings = std::mem::take(&mut self.var_bindings);

    for param in &arrow.params.items {
      self.declare_binding_pattern(&param.pattern);
    }
    for param in &arrow.params.items {
      collect_binding_pattern_names(&param.pattern, &mut self.var_bindings);
    }

    for stmt in &arrow.body.statements {
      self.visit_statement(stmt);
    }

    self.bindings = parent_bindings;
    self.var_bindings = parent_var_bindings;
  }

  fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
    let parent_bindings = std::mem::take(&mut self.bindings);
    walk::walk_for_statement(self, stmt);
    self.bindings = parent_bindings;
  }

  fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
    let parent_bindings = std::mem::take(&mut self.bindings);
    walk::walk_for_in_statement(self, stmt);
    self.bindings = parent_bindings;
  }

  fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
    let parent_bindings = std::mem::take(&mut self.bindings);
    walk::walk_for_of_statement(self, stmt);
    self.bindings = parent_bindings;
  }

  fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
    if decl.kind == VariableDeclarationKind::Var {
      for declarator in &decl.declarations {
        self.declare_var_binding_pattern(&declarator.id);
        if let Some(init) = &declarator.init {
          self.visit_expression(init);
        }
      }
    } else {
      // let or const: block-scoped, use regular declare
      for declarator in &decl.declarations {
        self.declare_binding_pattern(&declarator.id);
        if let Some(init) = &declarator.init {
          self.visit_expression(init);
        }
      }
    }
  }

  fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
    // Save current block-scoped bindings to restore after the block.
    // var_bindings persist (function-scoped).
    let parent_bindings = std::mem::take(&mut self.bindings);
    walk::walk_block_statement(self, block);
    self.bindings = parent_bindings;
  }

  fn visit_property_definition(&mut self, p: &PropertyDefinition<'a>) {
    if let Some(expr) = p.key.as_expression() {
      self.visit_expression(expr);
    }

    if let Some(value) = &p.value {
      self.visit_expression(value);
    }
  }
}

/// Collect all identifier names from a binding pattern into a set.
fn collect_binding_pattern_names(
  pattern: &BindingPattern<'_>,
  names: &mut HashSet<String>,
) {
  match pattern {
    BindingPattern::BindingIdentifier(ident) => {
      names.insert(ident.name.to_string());
    }
    BindingPattern::ObjectPattern(obj) => {
      for prop in &obj.properties {
        collect_binding_pattern_names(&prop.value, names);
      }
      if let Some(rest) = &obj.rest {
        collect_binding_pattern_names(&rest.argument, names);
      }
    }
    BindingPattern::ArrayPattern(arr) => {
      for elem in arr.elements.iter().flatten() {
        collect_binding_pattern_names(elem, names);
      }
      if let Some(rest) = &arr.rest {
        collect_binding_pattern_names(&rest.argument, names);
      }
    }
    BindingPattern::AssignmentPattern(assign) => {
      collect_binding_pattern_names(&assign.left, names);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_redeclare_valid() {
    assert_lint_ok! {
      NoRedeclare,
      "var a = 3; var b = function() { var a = 10; };",
      "var a = 3; a = 10;",
      "if (true) {\n    let b = 2;\n} else {    \nlet b = 3;\n}",
      "class C {
        constructor(a: string) {}
      }
      class D {
        constructor(a: string) {}
      }",

      // https://github.com/denoland/deno_lint/issues/615
      "class T { #foo(x) {} #bar(x) {} }",
      r#"
      async function test(t) {
        await t.step("one", async () => {
          const err = await assertRejects(() => foo());
        });
        await t.step("two", async () => {
          const err = await assertRejects(() => bar());
        });
      }
      "#,
      r#"
      async function build(config) {
        const env = {};
        for (const key of Object.keys(env)) {
          delete env[key];
        }
        if (config.cacheConfig !== undefined) {
          if (cache) {
            const key = new URL("https://example.com/" + config.cacheConfig.key);
            await cache.match(key);
          }
        }
      }
      "#,
      r#"
      async function* runWithPipedLogs(result) {
        while (stdoutRead || stderrRead) {
          const { result, level } = await Promise.race([
            stdoutRead,
            stderrRead,
          ]);
          yield {
            level,
            message: result.value,
          };
        }
      }
      "#,
    };
  }

  #[test]
  fn no_redeclare_invalid() {
    assert_lint_err! {
      NoRedeclare,
      "var a = 3; var a = 10;": [{col: 15, message: MESSAGE}],
      "switch(foo) { case a: var b = 3;\ncase b: var b = 4}": [{col: 12, line: 2, message: MESSAGE}],
      "var a = 3; var a = 10;": [{col: 15, message: MESSAGE}],
      "var a = {}; var a = [];": [{col: 16, message: MESSAGE}],
      "var a; function a() {}": [{col: 16, message: MESSAGE}],
      "function a() {} function a() {}": [{col: 25, message: MESSAGE}],
      "var a = function() { }; var a = function() { }": [{col: 28, message: MESSAGE}],
      "var a = function() { }; var a = new Date();": [{col: 28, message: MESSAGE}],
      "var a; var a;": [{col: 11, message: MESSAGE}],
      "export var a; var a;": [{col: 18, message: MESSAGE}],
      "function f() { var a; var a; }": [{col: 26, message: MESSAGE}],
      "function f(a) { var a; }": [{col: 20, message: MESSAGE}],
      "function f() { var a; if (test) { var a; } }": [{col: 38, message: MESSAGE}],
      "for (var a, a;;);": [{col: 12, message: MESSAGE}],
      "let a; let a;": [{col: 11, message: MESSAGE}],
      "let a; const a = 0;": [{col: 13, message: MESSAGE}],
      "const a = 0; const a = 0;": [{col: 19, message: MESSAGE}],
      "if (test) { let a; let a; }": [{col: 23, message: MESSAGE}],
      "switch (test) { case 0: let a; let a; }": [{col: 35, message: MESSAGE}],
      "for (let a, a;;);": [{col: 12, message: MESSAGE}],
      "for (let [a, a] in xs);": [{col: 13, message: MESSAGE}],
      "function f() { let a; let a; }": [{col: 26, message: MESSAGE}],
      "function f(a) { let a; }": [{col: 20, message: MESSAGE}],
      "function f() { if (test) { let a; let a; } }": [{col: 38, message: MESSAGE}],
      "var a = 3; var a = 10; var a = 15;": [{col: 15, message: MESSAGE}, {col: 27, message: MESSAGE}],
      "function f(foo: number, foo: string) {}": [{line: 1, col: 24, message: MESSAGE}],
    }
  }
}
