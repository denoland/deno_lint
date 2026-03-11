// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::Span;
use deno_ast::oxc::syntax::scope::ScopeFlags;
use derive_more::Display;
use std::collections::HashSet;

#[derive(Debug)]
pub struct NoInnerDeclarations;

const CODE: &str = "no-inner-declarations";

#[derive(Display)]
enum NoInnerDeclarationsMessage {
  #[display(fmt = "Move {} declaration to {} root", _0, _1)]
  Move(String, String),
}

#[derive(Display)]
enum NoInnerDeclarationsHint {
  #[display(fmt = "Move the declaration up into the correct scope")]
  Move,
}

impl LintRule for NoInnerDeclarations {
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
    // Pass 1: collect all valid declaration positions
    let mut valid_visitor = ValidDeclsVisitor::new();
    valid_visitor.visit_program(program);

    // Pass 2: check for invalid declarations
    let mut checker = NoInnerDeclarationsChecker {
      context,
      valid_decls: valid_visitor.valid_decls,
      in_function: false,
    };
    checker.visit_program(program);
  }
}

struct ValidDeclsVisitor {
  valid_decls: HashSet<Span>,
}

impl ValidDeclsVisitor {
  fn new() -> Self {
    Self {
      valid_decls: HashSet::new(),
    }
  }

  fn check_stmts(&mut self, stmts: &[Statement]) {
    for stmt in stmts {
      match stmt {
        Statement::FunctionDeclaration(func) => {
          self.valid_decls.insert(func.span);
        }
        Statement::VariableDeclaration(var_decl) => {
          if var_decl.kind == VariableDeclarationKind::Var {
            self.valid_decls.insert(var_decl.span);
          }
        }
        _ => {}
      }
    }
  }
}

impl<'a> Visit<'a> for ValidDeclsVisitor {
  fn visit_program(&mut self, program: &Program<'a>) {
    self.check_stmts(&program.body);
    walk::walk_program(self, program);
  }

  fn visit_export_named_declaration(
    &mut self,
    decl: &ExportNamedDeclaration<'a>,
  ) {
    if let Some(declaration) = &decl.declaration {
      match declaration {
        Declaration::FunctionDeclaration(func) => {
          self.valid_decls.insert(func.span);
        }
        Declaration::VariableDeclaration(var_decl) => {
          if var_decl.kind == VariableDeclarationKind::Var {
            self.valid_decls.insert(var_decl.span);
          }
        }
        _ => {}
      }
    }
    walk::walk_export_named_declaration(self, decl);
  }

  fn visit_export_default_declaration(
    &mut self,
    decl: &ExportDefaultDeclaration<'a>,
  ) {
    if let ExportDefaultDeclarationKind::FunctionDeclaration(func) =
      &decl.declaration
    {
      self.valid_decls.insert(func.span);
    }
    walk::walk_export_default_declaration(self, decl);
  }

  fn visit_function(
    &mut self,
    func: &Function<'a>,
    flags: ScopeFlags,
  ) {
    if let Some(body) = &func.body {
      self.check_stmts(&body.statements);
    }
    walk::walk_function(self, func, flags);
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    self.check_stmts(&arrow.body.statements);
    walk::walk_arrow_function_expression(self, arrow);
  }
}

struct NoInnerDeclarationsChecker<'c, 'view> {
  context: &'c mut Context<'view>,
  valid_decls: HashSet<Span>,
  in_function: bool,
}

impl NoInnerDeclarationsChecker<'_, '_> {
  fn add_diagnostic(&mut self, span: Span, kind: &str) {
    let root = if self.in_function {
      "function"
    } else {
      "module"
    };

    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoInnerDeclarationsMessage::Move(kind.to_string(), root.to_string()),
      NoInnerDeclarationsHint::Move,
    );
  }
}

impl<'a> Visit<'a> for NoInnerDeclarationsChecker<'_, '_> {
  fn visit_function(
    &mut self,
    func: &Function<'a>,
    flags: ScopeFlags,
  ) {
    // Check if this is a function declaration (not expression)
    // Function declarations that are not in valid positions should be flagged.
    // We flag based on whether the span is in valid_decls.
    if func.r#type == FunctionType::FunctionDeclaration {
      if !self.valid_decls.contains(&func.span) {
        self.add_diagnostic(func.span, "function");
      }
    }

    let old = self.in_function;
    self.in_function = true;
    walk::walk_function(self, func, flags);
    self.in_function = old;
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    let old = self.in_function;
    self.in_function = true;
    walk::walk_arrow_function_expression(self, arrow);
    self.in_function = old;
  }

  fn visit_variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration<'a>,
  ) {
    if var_decl.kind == VariableDeclarationKind::Var
      && !self.valid_decls.contains(&var_decl.span)
    {
      self.add_diagnostic(var_decl.span, "variable");
    }

    walk::walk_variable_declaration(self, var_decl);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_inner_declarations_valid() {
    assert_lint_ok! {
      NoInnerDeclarations,
      "function doSomething() { }",
      "function doSomething() { function somethingElse() { } }",
      "(function() { function doSomething() { } }());",
      "function decl() { var fn = function expr() { }; }",
      "function decl(arg) { var fn; if (arg) { fn = function() { }; } }",
      "var x = {doSomething() {function doSomethingElse() {}}}",
      "function decl(arg) { var fn; if (arg) { fn = function expr() { }; } }",
      "function decl(arg) { var fn; if (arg) { fn = function expr() { }; } }",
      "if (test) { let x = 1; }",
      "if (test) { const x = 1; }",
      "var foo;",
      "var foo = 42;",
      "function doSomething() { var foo; }",
      "(function() { var foo; }());",
      "foo(() => { function bar() { } });",
      "var fn = () => {var foo;}",
      "var x = {doSomething() {var foo;}}",
      "export var foo;",
      "export function bar() {}",
      "export default function baz() {}",
      "exports.foo = () => {}",
      "exports.foo = function(){}",
      "module.exports = function foo(){}",
      "class Test { constructor() { function test() {} } }",
      "class Test { method() { function test() {} } }",
    };
  }

  #[test]
  fn no_inner_declarations_invalid() {
    assert_lint_err! {
      NoInnerDeclarations,

      // fn decls
      "if (test) { function doSomething() { } }": [
        {
          col: 12,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "if (foo)  function f(){} ": [
        {
          col: 10,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "function bar() { if (foo) function f(){}; }": [
        {
          col: 26,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "function doSomething() { do { function somethingElse() { } } while (test); }": [
        {
          col: 30,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "(function() { if (test) { function doSomething() { } } }());": [
        {
          col: 26,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],

      // var decls
      "if (foo) var a; ": [
        {
          col: 9,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "if (foo) /* some comments */ var a; ": [
        {
          col: 29,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "function bar() { if (foo) var a; }": [
        {
          col: 26,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "if (foo){ var a; }": [
        {
          col: 10,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "while (test) { var foo; }": [
        {
          col: 15,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "module"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "function doSomething() { if (test) { var foo = 42; } }": [
        {
          col: 37,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "(function() { if (test) { var foo; } }());": [
        {
          col: 26,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "const doSomething = () => { if (test) { var foo = 42; } }": [
        {
          col: 40,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],

      // both
      "if (foo){ function f(){ if(bar){ var a; } } }": [
        {
          col: 10,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "module"),
          hint: NoInnerDeclarationsHint::Move,
        },
        {
          col: 33,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ],
      "if (foo) function f(){ if(bar) var a; } ": [
        {
          col: 9,
          message: variant!(NoInnerDeclarationsMessage, Move, "function", "module"),
          hint: NoInnerDeclarationsHint::Move,
        },
        {
          col: 31,
          message: variant!(NoInnerDeclarationsMessage, Move, "variable", "function"),
          hint: NoInnerDeclarationsHint::Move,
        }
      ]
    };
  }
}
