use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::ast;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoInnerDeclarations;

impl LintRule for NoInnerDeclarations {
  fn new() -> Box<Self> {
    Box::new(NoInnerDeclarations)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-inner-declarations"
  }

  fn lint_module(&self, context: &mut Context, module: &ast::Module) {
    let mut valid_visitor = ValidDeclsVisitor::new();
    valid_visitor.visit_module(module, module);
    let mut valid_decls = valid_visitor.valid_decls;
    valid_decls.dedup();
    let mut visitor = NoInnerDeclarationsVisitor::new(context, valid_decls);
    visitor.visit_module(module, module);
  }
}

struct ValidDeclsVisitor {
  pub valid_decls: Vec<Span>,
}

impl ValidDeclsVisitor {
  fn new() -> Self {
    Self {
      valid_decls: vec![],
    }
  }
}

impl ValidDeclsVisitor {
  fn check_decl(&mut self, decl: &ast::Decl) {
    match decl {
      ast::Decl::Fn(fn_decl) => {
        self.valid_decls.push(fn_decl.span());
      }
      ast::Decl::Var(var_decl) => {
        if var_decl.kind == ast::VarDeclKind::Var {
          self.valid_decls.push(var_decl.span());
        }
      }
      _ => {}
    }
  }
}

impl Visit for ValidDeclsVisitor {
  noop_visit_type!();

  fn visit_module_item(&mut self, item: &ast::ModuleItem, parent: &dyn Node) {
    match item {
      ast::ModuleItem::ModuleDecl(module_decl) => match module_decl {
        ast::ModuleDecl::ExportDecl(decl_export) => {
          self.check_decl(&decl_export.decl)
        }
        ast::ModuleDecl::ExportDefaultDecl(default_export) => {
          if let ast::DefaultDecl::Fn(fn_expr) = &default_export.decl {
            self.valid_decls.push(fn_expr.span());
          }
        }
        _ => {}
      },
      ast::ModuleItem::Stmt(module_stmt) => {
        if let ast::Stmt::Decl(decl) = module_stmt {
          self.check_decl(decl)
        }
      }
    }

    swc_ecmascript::visit::visit_module_item(self, item, parent);
  }

  fn visit_function(&mut self, function: &ast::Function, parent: &dyn Node) {
    if let Some(block) = &function.body {
      for stmt in &block.stmts {
        if let ast::Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
    swc_ecmascript::visit::visit_function(self, function, parent);
  }

  fn visit_fn_decl(&mut self, fn_decl: &ast::FnDecl, parent: &dyn Node) {
    if let Some(block) = &fn_decl.function.body {
      for stmt in &block.stmts {
        if let ast::Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
    swc_ecmascript::visit::visit_fn_decl(self, fn_decl, parent);
  }

  fn visit_fn_expr(&mut self, fn_expr: &ast::FnExpr, parent: &dyn Node) {
    if let Some(block) = &fn_expr.function.body {
      for stmt in &block.stmts {
        if let ast::Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
    swc_ecmascript::visit::visit_fn_expr(self, fn_expr, parent);
  }

  fn visit_arrow_expr(
    &mut self,
    arrow_expr: &ast::ArrowExpr,
    parent: &dyn Node,
  ) {
    if let ast::BlockStmtOrExpr::BlockStmt(block) = &arrow_expr.body {
      for stmt in &block.stmts {
        if let ast::Stmt::Decl(decl) = stmt {
          self.check_decl(decl);
        }
      }
    }
    swc_ecmascript::visit::visit_arrow_expr(self, arrow_expr, parent);
  }
}

struct NoInnerDeclarationsVisitor<'c> {
  context: &'c mut Context,
  valid_decls: Vec<Span>,
  in_function: bool,
}

impl<'c> NoInnerDeclarationsVisitor<'c> {
  fn new(context: &'c mut Context, valid_decls: Vec<Span>) -> Self {
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

    self.context.add_diagnostic(
      span,
      "no-inner-declarations",
      format!("Move {} declaration to {} root", kind, root),
    );
  }
}

impl<'c> Visit for NoInnerDeclarationsVisitor<'c> {
  noop_visit_type!();

  fn visit_arrow_expr(&mut self, fn_: &ast::ArrowExpr, parent: &dyn Node) {
    let old = self.in_function;
    self.in_function = true;
    swc_ecmascript::visit::visit_arrow_expr(self, fn_, parent);
    self.in_function = old;
  }

  fn visit_function(&mut self, fn_: &ast::Function, parent: &dyn Node) {
    let old = self.in_function;
    self.in_function = true;
    swc_ecmascript::visit::visit_function(self, fn_, parent);
    self.in_function = old;
  }

  fn visit_fn_decl(&mut self, decl: &ast::FnDecl, parent: &dyn Node) {
    let span = decl.span();

    if !self.valid_decls.contains(&span) {
      self.add_diagnostic(span, "function");
    }

    swc_ecmascript::visit::visit_fn_decl(self, decl, parent);
  }

  fn visit_var_decl(&mut self, decl: &ast::VarDecl, parent: &dyn Node) {
    let span = decl.span();

    if decl.kind == ast::VarDeclKind::Var && !self.valid_decls.contains(&span) {
      self.add_diagnostic(span, "variable");
    }

    swc_ecmascript::visit::visit_var_decl(self, decl, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_inner_declarations_ok() {
    assert_lint_ok_n::<NoInnerDeclarations>(vec![
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
    ]);
  }

  #[test]
  fn no_inner_declarations_err() {
    // fn decls
    assert_lint_err::<NoInnerDeclarations>(
      "if (test) { function doSomething() { } }",
      12,
    );
    assert_lint_err::<NoInnerDeclarations>("if (foo)  function f(){} ", 10);
    assert_lint_err::<NoInnerDeclarations>(
      "function bar() { if (foo) function f(){}; }",
      26,
    );
    assert_lint_err::<NoInnerDeclarations>("function doSomething() { do { function somethingElse() { } } while (test); }", 30);
    assert_lint_err::<NoInnerDeclarations>(
      "(function() { if (test) { function doSomething() { } } }());",
      26,
    );

    // var decls
    assert_lint_err::<NoInnerDeclarations>("if (foo) var a; ", 9);
    assert_lint_err::<NoInnerDeclarations>(
      "if (foo) /* some comments */ var a; ",
      29,
    );
    assert_lint_err::<NoInnerDeclarations>(
      "function bar() { if (foo) var a; }",
      26,
    );
    assert_lint_err::<NoInnerDeclarations>("if (foo){ var a; }", 10);
    assert_lint_err::<NoInnerDeclarations>("while (test) { var foo; }", 15);
    assert_lint_err::<NoInnerDeclarations>(
      "function doSomething() { if (test) { var foo = 42; } }",
      37,
    );
    assert_lint_err::<NoInnerDeclarations>(
      "(function() { if (test) { var foo; } }());",
      26,
    );
    assert_lint_err::<NoInnerDeclarations>(
      "const doSomething = () => { if (test) { var foo = 42; } }",
      40,
    );

    // both
    assert_lint_err_n::<NoInnerDeclarations>(
      "if (foo){ function f(){ if(bar){ var a; } } }",
      vec![10, 33],
    );
    assert_lint_err_n::<NoInnerDeclarations>(
      "if (foo) function f(){ if(bar) var a; } ",
      vec![9, 31],
    );
  }
}
