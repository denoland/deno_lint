// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use std::collections::HashSet;
use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmtOrExpr, Decl, DefaultDecl, FnDecl, FnExpr, Function,
  ModuleDecl, ModuleItem, Script, Stmt, VarDecl, VarDeclKind,
};
use swc_ecmascript::visit::{
  noop_visit_type, Node, Visit, VisitAll, VisitAllWith, VisitWith,
};

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
  fn new() -> Box<Self> {
    Box::new(NoInnerDeclarations)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut valid_visitor = ValidDeclsVisitor::new();
    match program {
      ProgramRef::Module(ref m) => {
        m.visit_all_with(&DUMMY_NODE, &mut valid_visitor)
      }
      ProgramRef::Script(ref s) => {
        s.visit_all_with(&DUMMY_NODE, &mut valid_visitor)
      }
    }

    let mut visitor =
      NoInnerDeclarationsVisitor::new(context, valid_visitor.valid_decls);
    match program {
      ProgramRef::Module(ref m) => m.visit_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows variable or function definitions in nested blocks

Function declarations in nested blocks can lead to less readable code and 
potentially unexpected results due to compatibility issues in different javascript
runtimes.  This does not apply to named or anonymous functions which are valid
in a nested block context.

Variables declared with `var` in nested blocks can also lead to less readable
code.  Because these variables are hoisted to the module root, it is best to 
declare them there for clarity.  Note that variables declared with `let` or
`const` are block scoped and therefore this rule does not apply to them.
    
### Invalid:
```typescript
if (someBool) { 
  function doSomething() {}
}

function someFunc(someVal:number): void {
  if (someVal > 4) {
    var a = 10;
  }
}
```

### Valid:
```typescript
function doSomething() {}
if (someBool) {}

var a = 10;
function someFunc(someVal:number): void {
  var foo = true;
  if (someVal > 4) {
    let b = 10;
    const fn = function doSomethingElse() {}
  }
}
```
"#
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
}

impl ValidDeclsVisitor {
  fn check_decl(&mut self, decl: &Decl) {
    match decl {
      Decl::Fn(fn_decl) => {
        self.valid_decls.insert(fn_decl.span());
      }
      Decl::Var(var_decl) => {
        if var_decl.kind == VarDeclKind::Var {
          self.valid_decls.insert(var_decl.span());
        }
      }
      _ => {}
    }
  }
}

impl VisitAll for ValidDeclsVisitor {
  noop_visit_type!();

  fn visit_script(&mut self, item: &Script, _: &dyn Node) {
    for stmt in &item.body {
      if let Stmt::Decl(decl) = stmt {
        self.check_decl(decl)
      }
    }
  }

  fn visit_module_item(&mut self, item: &ModuleItem, _: &dyn Node) {
    match item {
      ModuleItem::ModuleDecl(module_decl) => match module_decl {
        ModuleDecl::ExportDecl(decl_export) => {
          self.check_decl(&decl_export.decl)
        }
        ModuleDecl::ExportDefaultDecl(default_export) => {
          if let DefaultDecl::Fn(fn_expr) = &default_export.decl {
            self.valid_decls.insert(fn_expr.span());
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
  }

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    if let Some(block) = &function.body {
      for stmt in &block.stmts {
        if let Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
    if let Some(block) = &fn_decl.function.body {
      for stmt in &block.stmts {
        if let Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, _: &dyn Node) {
    if let Some(block) = &fn_expr.function.body {
      for stmt in &block.stmts {
        if let Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    if let BlockStmtOrExpr::BlockStmt(block) = &arrow_expr.body {
      for stmt in &block.stmts {
        if let Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
  }
}

struct NoInnerDeclarationsVisitor<'c> {
  context: &'c mut Context,
  valid_decls: HashSet<Span>,
  in_function: bool,
}

impl<'c> NoInnerDeclarationsVisitor<'c> {
  fn new(context: &'c mut Context, valid_decls: HashSet<Span>) -> Self {
    Self {
      context,
      valid_decls,
      in_function: false,
    }
  }
}

impl<'c> NoInnerDeclarationsVisitor<'c> {
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

impl<'c> Visit for NoInnerDeclarationsVisitor<'c> {
  noop_visit_type!();

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    let old = self.in_function;
    self.in_function = true;
    arrow_expr.visit_children_with(self);
    self.in_function = old;
  }

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    let old = self.in_function;
    self.in_function = true;
    function.visit_children_with(self);
    self.in_function = old;
  }

  fn visit_fn_decl(&mut self, decl: &FnDecl, _: &dyn Node) {
    let span = decl.span();

    if !self.valid_decls.contains(&span) {
      self.add_diagnostic(span, "function");
    }

    decl.visit_children_with(self);
  }

  fn visit_var_decl(&mut self, decl: &VarDecl, _: &dyn Node) {
    let span = decl.span();

    if decl.kind == VarDeclKind::Var && !self.valid_decls.contains(&span) {
      self.add_diagnostic(span, "variable");
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
