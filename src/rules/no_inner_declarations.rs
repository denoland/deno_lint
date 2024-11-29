// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::tags::{self, Tags};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::{
  ArrowExpr, BlockStmtOrExpr, Constructor, Decl, DefaultDecl, FnDecl, Function,
  ModuleDecl, ModuleItem, Script, Stmt, VarDecl, VarDeclKind,
};
use deno_ast::swc::visit::{noop_visit_type, Visit, VisitWith};
use deno_ast::SourceRange;
use deno_ast::SourceRangedForSpanned;
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

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut valid_visitor = ValidDeclsVisitor::new();
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut valid_visitor),
      ProgramRef::Script(s) => s.visit_with(&mut valid_visitor),
    }

    let mut visitor =
      NoInnerDeclarationsVisitor::new(context, valid_visitor.valid_decls);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_inner_declarations.md")
  }
}

struct ValidDeclsVisitor {
  valid_decls: HashSet<SourceRange>,
}

impl ValidDeclsVisitor {
  fn new() -> Self {
    Self {
      valid_decls: HashSet::new(),
    }
  }
}

impl ValidDeclsVisitor {
  fn check_stmts(&mut self, stmts: &[Stmt]) {
    for stmt in stmts {
      if let Stmt::Decl(decl) = stmt {
        self.check_decl(decl);
      }
    }
  }

  fn check_decl(&mut self, decl: &Decl) {
    match decl {
      Decl::Fn(fn_decl) => {
        self.valid_decls.insert(fn_decl.range());
      }
      Decl::Var(var_decl) => {
        if var_decl.kind == VarDeclKind::Var {
          self.valid_decls.insert(var_decl.range());
        }
      }
      _ => {}
    }
  }
}

impl Visit for ValidDeclsVisitor {
  noop_visit_type!();

  fn visit_script(&mut self, item: &Script) {
    for stmt in &item.body {
      if let Stmt::Decl(decl) = stmt {
        self.check_decl(decl)
      }
    }
    item.visit_children_with(self);
  }

  fn visit_module_item(&mut self, item: &ModuleItem) {
    match item {
      ModuleItem::ModuleDecl(module_decl) => match module_decl {
        ModuleDecl::ExportDecl(decl_export) => {
          self.check_decl(&decl_export.decl)
        }
        ModuleDecl::ExportDefaultDecl(default_export) => {
          if let DefaultDecl::Fn(fn_expr) = &default_export.decl {
            self.valid_decls.insert(fn_expr.range());
          }
        }
        _ => {}
      },
      ModuleItem::Stmt(module_stmt) => {
        if let Stmt::Decl(decl) = module_stmt {
          self.check_decl(decl)
        }
      }
    }
    item.visit_children_with(self);
  }

  fn visit_function(&mut self, function: &Function) {
    if let Some(block) = &function.body {
      self.check_stmts(&block.stmts);
    }
    function.visit_children_with(self);
  }

  fn visit_constructor(&mut self, constructor: &Constructor) {
    if let Some(block) = &constructor.body {
      self.check_stmts(&block.stmts);
    }
    constructor.visit_children_with(self);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    if let BlockStmtOrExpr::BlockStmt(block) = &*arrow_expr.body {
      self.check_stmts(&block.stmts);
    }
    arrow_expr.visit_children_with(self);
  }
}

struct NoInnerDeclarationsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  valid_decls: HashSet<SourceRange>,
  in_function: bool,
}

impl<'c, 'view> NoInnerDeclarationsVisitor<'c, 'view> {
  fn new(
    context: &'c mut Context<'view>,
    valid_decls: HashSet<SourceRange>,
  ) -> Self {
    Self {
      context,
      valid_decls,
      in_function: false,
    }
  }
}

impl<'c, 'view> NoInnerDeclarationsVisitor<'c, 'view> {
  fn add_diagnostic(&mut self, range: SourceRange, kind: &str) {
    let root = if self.in_function {
      "function"
    } else {
      "module"
    };

    self.context.add_diagnostic_with_hint(
      range,
      CODE,
      NoInnerDeclarationsMessage::Move(kind.to_string(), root.to_string()),
      NoInnerDeclarationsHint::Move,
    );
  }
}

impl<'c, 'view> Visit for NoInnerDeclarationsVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    let old = self.in_function;
    self.in_function = true;
    arrow_expr.visit_children_with(self);
    self.in_function = old;
  }

  fn visit_function(&mut self, function: &Function) {
    let old = self.in_function;
    self.in_function = true;
    function.visit_children_with(self);
    self.in_function = old;
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl) {
    let range = decl.range();

    if !self.valid_decls.contains(&range) {
      self.add_diagnostic(range, "function");
    }

    decl.visit_children_with(self);
  }

  fn visit_var_decl(&mut self, decl: &VarDecl) {
    let range = decl.range();

    if decl.kind == VarDeclKind::Var && !self.valid_decls.contains(&range) {
      self.add_diagnostic(range, "variable");
    }

    decl.visit_children_with(self);
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
