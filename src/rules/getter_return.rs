// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::BlockStmt;
use crate::swc_ecma_ast::Class;
use crate::swc_ecma_ast::ClassMember;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::ExprOrSuper;
use crate::swc_ecma_ast::GetterProp;
use crate::swc_ecma_ast::MethodKind;
use crate::swc_ecma_ast::Stmt;
use swc_atoms::JsWord;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct GetterReturn;

impl LintRule for GetterReturn {
  fn new() -> Box<Self> {
    Box::new(GetterReturn)
  }

  fn code(&self) -> &'static str {
    "getter-return"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = GetterReturnVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct GetterReturnVisitor {
  context: Arc<Context>,
}

impl GetterReturnVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn return_has_arg(&self, return_stmt: &swc_ecma_ast::ReturnStmt) -> bool {
    return_stmt.arg.is_some()
  }

  fn stmts_have_return(&self, stmts: &[Stmt]) -> bool {
    for stmt in stmts {
      if let Stmt::Return(return_stmt) = stmt {
        return self.return_has_arg(return_stmt);
      }
    }
    false
  }

  fn check_if_stmt(&self, if_stmt: &swc_ecma_ast::IfStmt) -> bool {
    if if_stmt.alt.is_none() {
      return false;
    }
    if let Stmt::Block(if_block) = &*if_stmt.cons {
      if !self.stmts_have_return(&if_block.stmts) {
        return false;
      }
      let mut alt_stmt = &if_stmt.alt;
      loop {
        if let Some(if_alt_block) = alt_stmt {
          if let Stmt::If(else_if) = &**if_alt_block {
            if else_if.alt.is_none() {
              return false;
            } else {
              alt_stmt = &else_if.alt;
            }
            if let Stmt::Block(else_if_block) = &*else_if.cons {
              if !self.stmts_have_return(&else_if_block.stmts) {
                return false;
              }
            }
          } else if let Stmt::Block(else_block) = &**if_alt_block {
            if self.stmts_have_return(&else_block.stmts) {
              break;
            } else {
              return false;
            }
          }
        }
      }
    }
    true
  }

  fn check_switch_stmt(&self, switch_stmt: &swc_ecma_ast::SwitchStmt) -> bool {
    for case in &switch_stmt.cases {
      if !case.cons.is_empty() && !self.stmts_have_return(&case.cons) {
        return false;
      }
    }
    true
  }

  fn check_block_stmt(&self, block_stmt: &BlockStmt, span: swc_common::Span) {
    if !block_stmt.stmts.iter().any(|s| match s {
      Stmt::If(if_stmt) => self.check_if_stmt(if_stmt),
      Stmt::Switch(switch_stmt) => self.check_switch_stmt(switch_stmt),
      Stmt::Return(return_stmt) => self.return_has_arg(return_stmt),
      _ => false,
    }) {
      self.context.add_diagnostic(
        span,
        "getter-return",
        "Getter requires a return",
      );
    }
  }
}

impl Visit for GetterReturnVisitor {
  fn visit_class(&mut self, class: &Class, _parent: &dyn Node) {
    for member in &class.body {
      match member {
        ClassMember::Method(class_method) => {
          if class_method.kind == MethodKind::Getter {
            if let Some(block_stmt) = &class_method.function.body {
              self.check_block_stmt(block_stmt, class_method.span);
            }
          }
        }
        ClassMember::PrivateMethod(private_method) => {
          if private_method.kind == MethodKind::Getter {
            if let Some(block_stmt) = &private_method.function.body {
              self.check_block_stmt(block_stmt, private_method.span);
            }
          }
        }
        _ => {}
      }
    }
  }

  fn visit_getter_prop(
    &mut self,
    getter_prop: &GetterProp,
    _parent: &dyn Node,
  ) {
    if let Some(block_stmt) = &getter_prop.body {
      self.check_block_stmt(block_stmt, getter_prop.span);
    }
  }

  fn visit_call_expr(
    &mut self,
    call_expr: &swc_ecma_ast::CallExpr,
    _parent: &dyn Node,
  ) {
    if call_expr.args.len() != 3 {
      return;
    }
    if let ExprOrSuper::Expr(callee_expr) = &call_expr.callee {
      if let Expr::Member(member) = &**callee_expr {
        if let ExprOrSuper::Expr(member_obj) = &member.obj {
          if let Expr::Ident(ident) = &**member_obj {
            if ident.sym != JsWord::from("Object") {
              return;
            }
          }
        }
        if let Expr::Ident(ident) = &*member.prop {
          if ident.sym != JsWord::from("defineProperty") {
            return;
          }
        }
      }
    }
    if let Expr::Object(obj_expr) = &*call_expr.args[2].expr {
      for prop in obj_expr.props.iter() {
        if let swc_ecma_ast::PropOrSpread::Prop(prop_expr) = prop {
          if let swc_ecma_ast::Prop::KeyValue(kv_prop) = &**prop_expr {
            if let swc_ecma_ast::PropName::Ident(ident) = &kv_prop.key {
              if ident.sym != JsWord::from("get") {
                return;
              }
              if let Expr::Fn(fn_expr) = &*kv_prop.value {
                if let Some(body) = &fn_expr.function.body {
                  self.check_block_stmt(&body, ident.span);
                }
              } else if let Expr::Arrow(arrow_expr) = &*kv_prop.value {
                if let swc_ecma_ast::BlockStmtOrExpr::BlockStmt(block_stmt) =
                  &arrow_expr.body
                {
                  self.check_block_stmt(&block_stmt, ident.span);
                }
              }
            }
          } else if let swc_ecma_ast::Prop::Method(method_prop) = &**prop_expr {
            if let swc_ecma_ast::PropName::Ident(ident) = &method_prop.key {
              if ident.sym != JsWord::from("get") {
                return;
              }
              if let Some(body) = &method_prop.function.body {
                self.check_block_stmt(&body, ident.span);
              }
            }
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
  fn getter_return_valid() {
    assert_lint_ok::<GetterReturn>("let foo = { get bar(){return true;} };");
    assert_lint_ok::<GetterReturn>("class foo { get bar(){return true;} }");
    assert_lint_ok::<GetterReturn>(
      "class foo { get bar(){if(baz){return true;} else {return false;} } }",
    );
    assert_lint_ok::<GetterReturn>("class foo { get(){return true;} }");
    assert_lint_ok::<GetterReturn>(
      r#"Object.defineProperty(foo, "bar", { get: function () {return true;}});"#,
    );
    assert_lint_ok::<GetterReturn>(
      r#"Object.defineProperty(foo, "bar",
         { get: function () { ~function (){ return true; }();return true;}});"#,
    );
    assert_lint_ok::<GetterReturn>(
      r#"Object.defineProperties(foo,
         { bar: { get: function () {return true;}} });"#,
    );
    assert_lint_ok::<GetterReturn>(
      r#"Object.defineProperties(foo,
         { bar: { get: function () { ~function (){ return true; }(); return true;}} });"#,
    );
    assert_lint_ok::<GetterReturn>(
      r#"
        let get = function(){};
        let get = function(){ return true; };
        let foo = { bar(){} };
        let foo = { bar(){ return true; } };
        let foo = { bar: function(){} };
        let foo = { bar: function(){return;} };
        let foo = { bar: function(){return true;} };
        let foo = { get: function () {} };
        let foo = { get: () => {}};
    "#,
    );
  }
  #[test]
  fn getter_return_invalid() {
    assert_lint_err::<GetterReturn>("const foo = { get getter() {} };", 14);
    assert_lint_err::<GetterReturn>(
      "const foo = { get bar() { ~function () {return true;}} };",
      14,
    );
    assert_lint_err::<GetterReturn>(
      "const foo = { get bar(){if(baz) {return true;}} };",
      14,
    );
    assert_lint_err::<GetterReturn>(
      "const foo = { get bar() { return; } };",
      14,
    );
    assert_lint_err::<GetterReturn>("class foo { get bar(){} }", 12);
    assert_lint_err::<GetterReturn>(
      "class foo { get bar(){ if (baz) { return true; }}}",
      12,
    );
    assert_lint_err::<GetterReturn>(
      "class foo { get bar(){ ~function () { return true; }()}}",
      12,
    );
    assert_lint_err::<GetterReturn>(
      "Object.defineProperty(foo, 'bar', { get: function (){}});",
      36,
    );
    assert_lint_err::<GetterReturn>(
      "Object.defineProperty(foo, 'bar', { get: function getfoo (){}});",
      36,
    );
    assert_lint_err::<GetterReturn>(
      "Object.defineProperty(foo, 'bar', { get(){} });",
      36,
    );
    assert_lint_err::<GetterReturn>(
      "Object.defineProperty(foo, 'bar', { get: () => {}});",
      36,
    );
    assert_lint_err::<GetterReturn>(
      r#"Object.defineProperty(foo, "bar", { get: function (){if(bar) {return true;}}});"#,
      36,
    );
    assert_lint_err::<GetterReturn>(
      r#"Object.defineProperty(foo, "bar", { get: function (){ ~function () { return true; }()}});"#,
      36,
    );
    assert_lint_err_n::<GetterReturn>(
      "class b { get getterA() {} private get getterB() {} }",
      vec![10, 27],
    );
  }
}
